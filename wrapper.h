#include <dlfcn.h>
#include <stdlib.h>

#ifdef COLLECTD_55
    #include <collectd/liboconfig/oconfig.h>
    #include <collectd/core/daemon/plugin.h>
#endif

#ifdef COLLECTD_54
    #include <collectd/core/plugin.h>
#endif

#ifdef COLLECTD_57
    #include <collectd/core/daemon/plugin.h>
#endif
