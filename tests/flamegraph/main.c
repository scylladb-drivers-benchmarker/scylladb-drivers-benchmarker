#include <assert.h>
#include <stdio.h>
#include <stdlib.h>

__attribute__((noinline)) int goo(int x);

__attribute__((noinline)) int foo(int x) {
    return goo(x - 1) + 1;
}

__attribute__((noinline)) int goo(int x) {
    if (x <= 1) {
        return x;
    }

    return foo(x - 1) + 1;
}

int main(int argc, char* argv[]) {
    if (argc != 2) {
        printf("Usage: %s <N>", argv[0]);
        return -1;
    }

    int n = atoi(argv[1]);
    int result = 0;
    for (int i = 0; i <= n; ++i) {
        int g = goo(i);
        result = result < g ? g : result;
    }
    assert(result == n);
}
