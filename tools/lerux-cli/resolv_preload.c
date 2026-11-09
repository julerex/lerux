/* QEMU's libslirp reads the first nameserver with fopen64("/etc/resolv.conf").
 * LERUX_RESOLV_CONF names the file those opens should see. Every other path
 * is unchanged. The smoke sets LD_PRELOAD on the QEMU process alone.
 */
#define _GNU_SOURCE

#include <dlfcn.h>
#include <pthread.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static FILE *(*real_fopen)(const char *, const char *);
static FILE *(*real_fopen64)(const char *, const char *);

static void init(void) {
    real_fopen = dlsym(RTLD_NEXT, "fopen");
    real_fopen64 = dlsym(RTLD_NEXT, "fopen64");
}

static const char *resolv_path(const char *path) {
    const char *alt;

    if (path == NULL || strcmp(path, "/etc/resolv.conf") != 0) {
        return path;
    }
    alt = getenv("LERUX_RESOLV_CONF");
    if (alt == NULL || alt[0] == '\0') {
        return path;
    }
    return alt;
}

static pthread_once_t once = PTHREAD_ONCE_INIT;

FILE *fopen(const char *path, const char *mode) {
    pthread_once(&once, init);
    if (real_fopen == NULL) {
        return NULL;
    }
    return real_fopen(resolv_path(path), mode);
}

FILE *fopen64(const char *path, const char *mode) {
    pthread_once(&once, init);
    if (real_fopen64 == NULL) {
        return NULL;
    }
    return real_fopen64(resolv_path(path), mode);
}
