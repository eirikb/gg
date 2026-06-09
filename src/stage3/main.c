#include "const.h"
#include <stdio.h>
#include <string.h>
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
    long dataSize = 0;
    long messageSize = 0;
    long totalSize = 0;
    int p = 0;

    do {
        const long res = read(sock, buffer, bufferSize - 1);
        if (res <= 0) {
            printf("\nDownload interrupted\n");
            fclose(f);
            remove("stage4.tmp");
            return 1;
        }
        if (dataSize == 0) {
            buffer[res] = '\0';
            const char *statusSpace = strncmp(buffer, "HTTP/", 5) == 0 ? strchr(buffer, ' ') : NULL;
            const int status = statusSpace ? atoi(statusSpace + 1) : 0;
            if (status != 200) {
                printf("HTTP error %d from %s\n", status, connectHost);
                fclose(f);
                remove("stage4.tmp");
                return 1;
            }
            const char *clHeader = strstr(buffer, "Content-Length");
            const char *cl = clHeader ? strstr(clHeader, " ") : NULL;
            const char *end = strstr(buffer, "\r\n\r\n");
            if (cl == NULL || end == NULL) {
                printf("Unexpected HTTP response from %s\n", host);
                fclose(f);
                remove("stage4.tmp");
                return 1;
            }
            char *to = strstr(cl, "\r");
            dataSize = strtol(cl, &to, 10);
            messageSize = end - buffer + dataSize + 4;
            totalSize = messageSize;
            fwrite(end + 4, 1, res - (messageSize - dataSize), f);
            printf("0%%");
        } else {
            fwrite(buffer, 1, res, f);
        }
        messageSize -= res;
        const int np = 100 - (int) ((double) messageSize / (double) totalSize * 100);
        if (np != p) {
            p = np;
            if (p % 10 == 0) {
                printf("%d%%", p);
            } else {
                printf(".");
            }
            fflush(stdout);
        }
    } while (messageSize > 0);
    fclose(f);

    printf("\n");

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
