#ifdef COLLECTD_54
    #include <collectd/core/plugin.h>
    #include <collectd/core/utils_cache.h>
#else
    #include <collectd/liboconfig/oconfig.h>
    #include <collectd/core/daemon/plugin.h>
    #include <collectd/core/daemon/utils_cache.h>
#endif
