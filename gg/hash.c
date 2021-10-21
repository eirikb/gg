#include "sha512.c"
#include <stdio.h>

const int HASH_SIZE = 128;

void toHex(unsigned char *hash, int size, char *outputBuffer) {
  for (int i = 0; i < size; i++) {
    sprintf(outputBuffer + (i * 2), "%02x", hash[i]);
  }
  outputBuffer[size * 2] = 0;
}

void hashForFile(char *filenName, char *hash) {
  SHA512_CTX ctx;
  SHA512_Init(&ctx);
  const int bufSize = 32768;
  char buffer[32768];
  size_t bytesRead;
  FILE *fp = fopen(filenName, "rb");
  while ((bytesRead = fread(buffer, 1, bufSize, fp)) > 0) {
    SHA512_Update(&ctx, buffer, bytesRead);
  }
  fclose(fp);

  unsigned char digest[SHA512_DIGEST_LENGTH];
  SHA512_Final(digest, &ctx);
  toHex(digest, SHA512_DIGEST_LENGTH, hash);
}
