#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "sha256-512/SHA512.h"

const int HASH_SIZE = 128;

void toHex(const uint64_t *hash, int size, char *outputBuffer) {
    for (int i = 0; i < size; i++) {
        sprintf(outputBuffer + (i * 16), "%016" PRIx64, hash[i]);
    }
    outputBuffer[size * 16] = 0;
}

void hashForFile(const char *fileName, char *hash) {
    const int bufSize = 32768;
    unsigned char buffer[bufSize];
    size_t totalBytes = 0;
    size_t bytesRead;
    FILE *fp = fopen(fileName, "rb");

    fseek(fp, 0, SEEK_END);
    const size_t fileSize = ftell(fp);
    fseek(fp, 0, SEEK_SET);

    unsigned char *fileBuffer = malloc(fileSize);
    if (!fileBuffer) {
        printf("Memory allocation failed\n");
        fclose(fp);
        return;
    }

    while ((bytesRead = fread(buffer, 1, bufSize, fp)) > 0) {
        memcpy(fileBuffer + totalBytes, buffer, bytesRead);
        totalBytes += bytesRead;
    }
    fclose(fp);

    uint64_t *digest = SHA512Hash(fileBuffer, fileSize);

    toHex(digest, HASH_ARRAY_LEN, hash);

    free(digest);
    free(fileBuffer);
}
