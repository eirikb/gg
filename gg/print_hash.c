#include "hash.c"

int main(int argc, char *argv[]) {
  if (argc != 2) {
    printf("Please provide a file name\n");
    return 1;
  }

  char hash[HASH_SIZE];
  hashForFile(argv[1], hash);
  printf("%s\n", hash);
  return 0;
}
