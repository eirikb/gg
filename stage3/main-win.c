#define WIN32_LEAN_AND_MEAN

#include "const.h"
#include "hash.c"
#include <Ws2tcpip.h>
#include <stdio.h>
#include <stdlib.h>
#include <winsock2.h>

#pragma comment(lib, "Ws2_32.lib")

#define DEFAULT_BUFLEN 65536

int __cdecl main() {
  WSADATA wsaData;
  SOCKET ConnectSocket = INVALID_SOCKET;
  struct addrinfo *result = NULL, *ptr = NULL, hints;
  char path[1000];
  snprintf(path, 1000, "/%s", hash);
  const char *host = "gg.eirikb.no";
  char header[1024];
  snprintf(header, sizeof(header), "GET %s HTTP/1.1\r\nHost: %s\r\n\r\n", path,
           host);
  size_t res;

  res = WSAStartup(MAKEWORD(2, 2), &wsaData);
  if (res != 0) {
    printf("WSAStartup failed with error: %d\n", res);
    return 1;
  }

  ZeroMemory(&hints, sizeof(hints));
  hints.ai_family = AF_UNSPEC;
  hints.ai_socktype = SOCK_STREAM;
  hints.ai_protocol = IPPROTO_TCP;

  res = getaddrinfo(host, "80", &hints, &result);
  if (res != 0) {
    printf("getaddrinfo failed with error: %d\n", res);
    WSACleanup();
    return 1;
  }

  for (ptr = result; ptr != NULL; ptr = ptr->ai_next) {
    ConnectSocket = socket(ptr->ai_family, ptr->ai_socktype, ptr->ai_protocol);
    if (ConnectSocket == INVALID_SOCKET) {
      printf("socket failed with error: %ld\n", WSAGetLastError());
      WSACleanup();
      return 1;
    }

    res = connect(ConnectSocket, ptr->ai_addr, (int)ptr->ai_addrlen);
    if (res == SOCKET_ERROR) {
      closesocket(ConnectSocket);
      ConnectSocket = INVALID_SOCKET;
      continue;
    }
    break;
  }

  freeaddrinfo(result);

  if (ConnectSocket == INVALID_SOCKET) {
    printf("Unable to connect to server!\n");
    WSACleanup();
    return 1;
  }

  res = send(ConnectSocket, header, (int)strlen(header), 0);
  if (res == SOCKET_ERROR) {
    printf("send failed with error: %d\n", WSAGetLastError());
    closesocket(ConnectSocket);
    WSACleanup();
    return 1;
  }

  res = shutdown(ConnectSocket, SD_SEND);
  if (res == SOCKET_ERROR) {
    printf("shutdown failed with error: %d\n", WSAGetLastError());
    closesocket(ConnectSocket);
    WSACleanup();
    return 1;
  }

  FILE *f;
  fopen_s(&f, "stage4", "wb");
  if (f == NULL) {
    printf("Error opening file!\n");
    return 1;
  }

  int p = 0;

  size_t message_size = 0;
  size_t data_size = 0;
  size_t total_size = 0;
  do {
    char buffer[DEFAULT_BUFLEN];
    res = recv(ConnectSocket, buffer, DEFAULT_BUFLEN, 0);
    if (data_size == 0) {
      char *cl = strstr(strstr(buffer, "Content-Length"), " ");
      char *to = strstr(cl, "\r");
      data_size = strtol(cl, &to, 10);
      char *end = strstr(buffer, "\r\n\r\n");
      message_size = end - buffer + data_size + 4;
      total_size = message_size;
      fwrite(end + 4, 1, res - (message_size - data_size), f);
    } else {
      fwrite(buffer, sizeof(char), res, f);
    }
    message_size -= res;
    int np = 100 - (int)((double)message_size / (double)total_size * 100);
    if (np != p) {
      p = np;
      if (p % 10 == 0) {
        printf("%d%%", p);
      } else {
        printf(".");
      }
      fflush(stdout);
    }
  } while (message_size > 0);
  fclose(f);

  char newHash[SHA512_BLOCK_LENGTH + 1];
  hashForFile("stage4", newHash);
  if (strcmp(hash, newHash) != 0) {
    printf("Hash did not match :(\n");
    return 1;
  }

  return 0;
}
