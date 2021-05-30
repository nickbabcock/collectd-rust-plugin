#ifdef COLLECTD_PATH
    #include <liboconfig/oconfig.h>
    #include <daemon/plugin.h>
    #include <daemon/utils_cache.h>
#else
    #include <collectd/liboconfig/oconfig.h>
    #include <collectd/core/daemon/plugin.h>
    #include <collectd/core/daemon/utils_cache.h>
#endif
