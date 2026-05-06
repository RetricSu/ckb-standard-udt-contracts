typedef unsigned long size_t;

static int matches_allow(const unsigned char *data, size_t len) {
  const unsigned char expected[] = "allow";
  size_t expected_len = sizeof(expected) - 1;
  size_t index = 0;

  if (len != expected_len) {
    return 0;
  }

  for (index = 0; index < expected_len; index++) {
    if (data[index] != expected[index]) {
      return 0;
    }
  }
  return 1;
}

__attribute__((visibility("default"))) int eudt_authorize(
    const unsigned char *script_hash, const unsigned char *args,
    unsigned long args_len) {
  (void)script_hash;
  return matches_allow(args, args_len) ? 0 : 1;
}
