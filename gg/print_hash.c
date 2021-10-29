#include "hash.c"

int main(int argc, char *argv[]) {
  if (argc != 2) {
    printf("Please provide a file name\n");
    return 1;
  }

  char hash[128];
  hashForFile(argv[1], hash);
  printf("%s\n", hash);
  return 0;
}
