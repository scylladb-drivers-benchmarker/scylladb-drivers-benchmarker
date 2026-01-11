#include <assert.h>
#include <stdio.h>
#include <stdlib.h>

int foo(int x);

int goo(int x) {
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
    int result = goo(n);
    assert(result == n);
}
