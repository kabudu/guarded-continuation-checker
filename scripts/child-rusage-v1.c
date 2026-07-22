#define _GNU_SOURCE
#include <errno.h>
#include <stdio.h>
#include <stdlib.h>
#include <sys/resource.h>
#include <sys/types.h>
#include <sys/wait.h>
#include <unistd.h>

int main(int argc, char **argv) {
  if (argc < 2) {
    fputs("usage: child-rusage-v1 COMMAND [ARG ...]\n", stderr);
    return 2;
  }
  pid_t child = fork();
  if (child < 0) {
    perror("fork");
    return 2;
  }
  if (child == 0) {
    execvp(argv[1], &argv[1]);
    perror("execvp");
    _exit(127);
  }
  int status = 0;
  struct rusage usage;
  if (wait4(child, &status, 0, &usage) < 0) {
    perror("wait4");
    return 2;
  }
  if (usage.ru_maxrss < 0) {
    fputs("negative child peak RSS\n", stderr);
    return 2;
  }
#if defined(__APPLE__)
  const long long peak_rss_bytes = (long long)usage.ru_maxrss;
#else
  const long long peak_rss_bytes = (long long)usage.ru_maxrss * 1024LL;
#endif
  fprintf(stderr, "child_rusage_v1_peak_rss_bytes=%lld\n", peak_rss_bytes);
  if (WIFEXITED(status)) {
    return WEXITSTATUS(status);
  }
  if (WIFSIGNALED(status)) {
    return 128 + WTERMSIG(status);
  }
  return 2;
}
