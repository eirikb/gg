#include "const.h"
#include <stdio.h>
#include <string.h>
#include <strings.h>
#include "sha256-512/SHA512.h"
#include <netdb.h>
#include <netinet/in.h>
#include <stdlib.h>
#include <sys/socket.h>
#include <unistd.h>

/**
 * Downloads http://ggcmd.io/. That's it.
 * Using HTTP, not HTTPS. Hard coded checksum.
 **/

/* Honor http_proxy / all_proxy (the download is plain HTTP). Accepts
 * [http://]host[:port]; other schemes (socks, https) are ignored and we
 * connect directly. Credentials are not supported (no base64 here) and are
 * stripped, so an authenticating proxy will answer 407. */
static int parseProxy(char *host, size_t hostSize, char *port, size_t portSize) {
    const char *vars[] = {"http_proxy", "HTTP_PROXY", "all_proxy", "ALL_PROXY"};
    const char *v = NULL;
    for (size_t i = 0; i < sizeof(vars) / sizeof(vars[0]) && v == NULL; i++) {
        v = getenv(vars[i]);
        if (v != NULL && *v == '\0') {
            v = NULL;
        }
    }
    if (v == NULL) {
        return 0;
    }
    const char *p = strstr(v, "://");
    if (p != NULL) {
        if (strncmp(v, "http://", 7) != 0) {
            return 0;
        }
        p += 3;
    } else {
        p = v;
    }
    const char *at = strchr(p, '@');
    if (at != NULL) {
        printf("Proxy credentials are not supported, trying without\n");
        p = at + 1;
    }
    size_t hostLen = strcspn(p, ":/");
    if (hostLen == 0 || hostLen >= hostSize) {
        return 0;
    }
    memcpy(host, p, hostLen);
    host[hostLen] = '\0';
    p += hostLen;
    if (*p == ':') {
        p++;
        size_t portLen = strcspn(p, "/");
        if (portLen == 0 || portLen >= portSize) {
            return 0;
        }
        memcpy(port, p, portLen);
        port[portLen] = '\0';
    } else {
        snprintf(port, portSize, "80");
    }
    return 1;
}

static int fail(FILE *f) {
    fclose(f);
    remove("stage4.tmp");
    return 1;
}

/* stdout, stage2 drops stderr */
static void printHeaders(const char *buffer, long size) {
    if (size <= 0) {
        printf("(nothing received)\n");
        return;
    }
    const long max = size < 2000 ? size : 2000;
    for (long i = 0; i < max; i++) {
        if (buffer[i] != '\r') {
            putchar(buffer[i]);
        }
    }
    if (buffer[max - 1] != '\n') {
        printf("\n");
    }
}

/* Case-insensitive, some proxies lowercase the names */
static const char *findHeader(const char *buffer, long headerSize, const char *name) {
    const size_t nameLen = strlen(name);
    const char *line = strstr(buffer, "\r\n");
    while (line != NULL && line + 2 < buffer + headerSize) {
        line += 2;
        if (strncasecmp(line, name, nameLen) == 0 && line[nameLen] == ':') {
            const char *value = line + nameLen + 1;
            while (*value == ' ' || *value == '\t') {
                value++;
            }
            return value;
        }
        line = strstr(line, "\r\n");
    }
    return NULL;
}

/* Digits only, "12garbage" is not 12 */
static long contentLength(const char *buffer, long headerSize) {
    const char *v = findHeader(buffer, headerSize, "Content-Length");
    if (v == NULL || *v < '0' || *v > '9') {
        return -1;
    }
    long value = 0;
    for (; *v >= '0' && *v <= '9'; v++) {
        if (value > 100000000) {
            return -1;
        }
        value = value * 10 + (*v - '0');
    }
    while (*v == ' ' || *v == '\t') {
        v++;
    }
    return *v == '\r' ? value : -1;
}

int main() {
    const long bufferSize = 65536;
    const char *host = "ggcmd.z13.web.core.windows.net";

    char path[1000];
    snprintf(path, 1000, "/%s", hash);

    char proxyHost[256];
    char proxyPort[16];
    const int useProxy = parseProxy(proxyHost, sizeof(proxyHost), proxyPort, sizeof(proxyPort));
    const char *connectHost = useProxy ? proxyHost : host;
    const char *connectPort = useProxy ? proxyPort : "80";

    struct addrinfo hints;
    struct addrinfo *result = NULL;
    memset(&hints, 0, sizeof(hints));
    hints.ai_family = AF_UNSPEC;
    hints.ai_socktype = SOCK_STREAM;

    if (getaddrinfo(connectHost, connectPort, &hints, &result) != 0 || result == NULL) {
        printf("DNS lookup failed for %s\n", connectHost);
        return 1;
    }

    int sock = -1;
    for (struct addrinfo *ptr = result; ptr != NULL; ptr = ptr->ai_next) {
        sock = socket(ptr->ai_family, ptr->ai_socktype, ptr->ai_protocol);
        if (sock < 0) {
            continue;
        }
        if (connect(sock, ptr->ai_addr, ptr->ai_addrlen) == 0) {
            break;
        }
        close(sock);
        sock = -1;
    }
    freeaddrinfo(result);

    if (sock < 0) {
        printf("Connection to %s failed\n", connectHost);
        return 1;
    }

    FILE *f = fopen("stage4.tmp", "w");
    if (f == NULL) {
        printf("Error opening file!\n");
        exit(1);
    }

    char header[1200];
    if (useProxy) {
        // Through a proxy the request line must carry the absolute URI
        snprintf(header, sizeof(header),
                 "GET http://%s%s HTTP/1.1\r\nHost: %s\r\n\r\n", host, path, host);
    } else {
        snprintf(header, sizeof(header), "GET %s HTTP/1.1\r\nHost: %s\r\n\r\n",
                 path, host);
    }
    send(sock, header, strlen(header), 0);

    char buffer[bufferSize];
    /* The dots go to stdout, so anything parsing gg's output gets them too (#309) */
    const char *hideProgressEnv = getenv("GG_HIDE_DOWNLOAD_PROGRESS");
    const int showProgress = !(hideProgressEnv && *hideProgressEnv);

    /* px (#325) sends the headers in small pieces, so read until all are in */
    long have = 0;
    char *end = NULL;
    while (end == NULL) {
        if (have >= bufferSize - 1) {
            printf("HTTP headers from %s are too large\n", connectHost);
            printHeaders(buffer, have);
            return fail(f);
        }
        const long res = read(sock, buffer + have, bufferSize - 1 - have);
        if (res <= 0) {
            printf("Connection to %s closed before the HTTP headers were complete\n", connectHost);
            printHeaders(buffer, have);
            return fail(f);
        }
        have += res;
        buffer[have] = '\0';
        end = strstr(buffer, "\r\n\r\n");
    }
    const long headerSize = end - buffer + 4;

    const char *statusSpace = strncmp(buffer, "HTTP/", 5) == 0 ? strchr(buffer, ' ') : NULL;
    const int status = statusSpace ? atoi(statusSpace + 1) : 0;
    if (status != 200) {
        printf("HTTP error %d from %s\n", status, connectHost);
        printHeaders(buffer, headerSize - 2);
        return fail(f);
    }
    /* Chunk markers would end up in the file and just give a hash mismatch */
    if (findHeader(buffer, headerSize, "Transfer-Encoding") != NULL) {
        printf("Unsupported Transfer-Encoding in HTTP response from %s\n", connectHost);
        printHeaders(buffer, headerSize - 2);
        return fail(f);
    }
    const long dataSize = contentLength(buffer, headerSize);
    if (dataSize < 0) {
        printf("Missing or invalid Content-Length in HTTP response from %s\n", connectHost);
        printHeaders(buffer, headerSize - 2);
        return fail(f);
    }

    long written = have - headerSize < dataSize ? have - headerSize : dataSize;
    fwrite(buffer + headerSize, 1, written, f);
    if (showProgress) {
        printf("0%%");
    }

    int p = 0;
    while (1) {
        const int np = dataSize > 0 ? (int) ((double) written / (double) dataSize * 100) : 100;
        if (np != p) {
            p = np;
            if (showProgress) {
                if (p % 10 == 0) {
                    printf("%d%%", p);
                } else {
                    printf(".");
                }
                fflush(stdout);
            }
        }
        if (written >= dataSize) {
            break;
        }
        const long left = dataSize - written;
        const long res = read(sock, buffer, left < bufferSize ? left : bufferSize);
        if (res <= 0) {
            if (showProgress) {
                printf("\n");
            }
            printf("Download interrupted\n");
            return fail(f);
        }
        fwrite(buffer, 1, res, f);
        written += res;
    }
    fclose(f);

    if (showProgress) {
        printf("\n");
    }

    char newHash[129];
    extern void hashForFile(char *fileName, char *hash);

    hashForFile("stage4.tmp", newHash);
    if (strcmp(hash, newHash) != 0) {
        printf("Hash did not match :(\n");
        remove("stage4.tmp");
        return 1;
    }

    if (rename("stage4.tmp", "stage4") != 0) {
        printf("Failed to rename temp file\n");
        remove("stage4.tmp");
        return 1;
    }

    return 0;
}
