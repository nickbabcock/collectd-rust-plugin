#include <dlfcn.h>
#include <stdlib.h>

// If we're dealing with pre-collectd 5.7, we need to pull in another header
#ifndef COLLECTD_NEW
#include <collectd/liboconfig/oconfig.h>
#endif

#include <collectd/core/daemon/plugin.h>
