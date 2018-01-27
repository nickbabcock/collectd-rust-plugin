#[macro_use]
extern crate collectd_plugin;
extern crate failure;
extern crate serde;
#[macro_use]
extern crate serde_derive;

use collectd_plugin::{collectd_log, ConfigItem, LogLevel, Plugin, PluginCapabilities,
                      PluginManager, PluginRegistration, ValueList, Value};
use failure::Error;
use std::sync::Mutex;
use std::net::TcpStream;
use std::io::Write;
use std::borrow::Cow;
use std::ops::Deref;

/// Here is what our collectd config can look like:
///
/// ```
/// LoadPlugin write_graphite_rust
/// <Plugin write_graphite_rust>
///     <Node>
///         Name "localhost.1"
///         Address "127.0.0.1:20003"
///     </Node>
///     <Node>
///         Name "localhost.2"
///         Address "127.0.0.1:20004"
///         Prefix "iamprefix"
///     </Node>
/// </Plugin>
/// ```
#[derive(Deserialize, Debug, PartialEq, Default)]
#[serde(deny_unknown_fields)]
struct GraphiteConfig {
    #[serde(rename = "Node")]
    nodes: Vec<GraphiteNode>,
}

#[derive(Deserialize, Debug, PartialEq, Default)]
#[serde(rename_all = "PascalCase")]
#[serde(deny_unknown_fields)]
struct GraphiteNode {
    name: String,
    address: String,
    prefix: Option<String>,
}

struct GraphitePlugin<W: Write + Send> {
    // We need a mutex as writers aren't thread safe
    writer: Mutex<W>,
    prefix: Option<String>,
}

struct GraphiteManager;

impl PluginManager for GraphiteManager {
    fn name() -> &'static str {
        "write_graphite_rust"
    }

    fn plugins(config: Option<&[ConfigItem]>) -> Result<PluginRegistration, Error> {
        // Deserialize the collectd configuration into our configuration struct
        let config: GraphiteConfig =
            collectd_plugin::de::from_collectd(config.unwrap_or_else(Default::default))?;

        let config: Result<Vec<(String, Box<Plugin>)>, Error> = config.nodes
            .into_iter()
            .map(|x| {
                let plugin = GraphitePlugin {
                    writer: Mutex::new(TcpStream::connect(x.address)?),
                    prefix: x.prefix,
                };
                let bx: Box<Plugin> = Box::new(plugin);
                Ok((x.name.clone(), bx))
            })
            .collect();

        Ok(PluginRegistration::Multiple(config?))
    }
}

/// If necessary removes any characters from a string that have special meaning in graphite.
fn graphitize(s: &str) -> Cow<str> {
    let needs_modifying = s.chars().any(|x| x == '.' || x.is_whitespace() || x.is_control());
    if !needs_modifying {
        Cow::Borrowed(s)
    } else {
        let new_s: String = s.chars()
            .map(|x| if x == '.' || x.is_whitespace() || x.is_control() { '-' } else { x })
            .collect();
        Cow::Owned(new_s)
    }
}

impl<W: Write + Send> GraphitePlugin<W> {
    fn write_value(&mut self, mut line: String, val: Value, dt: &str) {
        line.push(' ');
        line.push_str(val.to_string().as_str());
        line.push(' ');
        line.push_str(dt);
        line.push('\n');

        // Finally, we get our exclusive lock on the tcp writer and send our data down the pipe. If
        // there is a failure, the proper response would be to try and allocate a new connection or
        // backoff. Instead we log the error.
        let mut w = self.writer.lock().unwrap();
        if let Err(ref e) = w.write(line.as_bytes()) {
            collectd_log(LogLevel::Error, e.to_string().as_str());
        }
    }
}

impl<W: Write + Send> Plugin for GraphitePlugin<W> {
    fn capabilities(&self) -> PluginCapabilities {
        PluginCapabilities::WRITE
    }

    fn write_values<'a>(&mut self, list: ValueList<'a>) -> Result<(), Error> {
        // We use a heap allocated string to construct data to send to graphite. Collectd doesn't
        // use the heap (preferring fixed size arrays). We could get the same behavior using the
        // ArrayString type from the arrayvec crate.
        let mut line = String::new();
        if let Some(ref prefix) = self.prefix {
            line.push_str(prefix.as_str());
            line.push('.');
        }
        line.push_str(graphitize(list.host).deref());
        line.push('.');
        line.push_str(graphitize(list.plugin).deref());

        if let Some(instance) = list.plugin_instance {
            line.push('-');
            line.push_str(graphitize(instance).deref());
        }

        line.push('.');
        line.push_str(graphitize(list.type_).deref());

        if let Some(type_instance) = list.type_instance {
            line.push('-');
            line.push_str(graphitize(type_instance).deref());
        }

        let dt = list.time.timestamp().to_string();

        // If there is only one value in the list we don't have to clone our premade string,
        // instead we can write it directly
        if list.values.len() == 1 {
            self.write_value(line, list.values[0].value, dt.as_str());
        } else {
            for v in list.values {
                let mut nv = line.clone();
                nv.push('.');
                nv.push_str(graphitize(v.name).deref());
                self.write_value(nv, v.value, dt.as_str());
            }
        }

        Ok(())
    }
}

collectd_plugin!(GraphiteManager);
