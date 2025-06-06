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

int main() {
    const long bufferSize = 65536;
    const char *host = "ggcmd.z13.web.core.windows.net";

    char path[1000];
    snprintf(path, 1000, "/%s", hash);

    struct sockaddr_in serv_addr;
    const int sock = socket(AF_INET, SOCK_STREAM, 0);

    struct hostent *server = gethostbyname(host);

    serv_addr.sin_family = AF_INET;
    serv_addr.sin_port = htons(80);
    memcpy(&serv_addr.sin_addr.s_addr, server->h_addr, server->h_length);

    if (connect(sock, (struct sockaddr *) &serv_addr, sizeof(serv_addr)) < 0) {
        printf("\nConnection Failed \n");
        return -1;
    }

    FILE *f = fopen("stage4.tmp", "w");
    if (f == NULL) {
        printf("Error opening file!\n");
        exit(1);
    }

    char header[1024];
    snprintf(header, sizeof(header), "GET %s HTTP/1.1\r\nHost: %s\r\n\r\n", path,
             host);
    send(sock, header, strlen(header), 0);

    char buffer[bufferSize];
    long dataSize = 0;
    long messageSize = 0;
    long totalSize = 0;
    int p = 0;

    do {
        const long res = read(sock, buffer, bufferSize);
        if (dataSize == 0) {
            const char *cl = strstr(strstr(buffer, "Content-Length"), " ");
            char *to = strstr(cl, "\r");
            dataSize = strtol(cl, &to, 10);
            const char *end = strstr(buffer, "\r\n\r\n");
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
