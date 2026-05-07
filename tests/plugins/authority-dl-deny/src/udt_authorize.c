typedef unsigned long size_t;

static int script_hash_is_nonzero(const unsigned char *script_hash) {
  size_t index = 0;
  for (index = 0; index < 32; index++) {
    if (script_hash[index] != 0) {
      return 1;
    }
  }
  return 0;
}

__attribute__((visibility("default"))) int udt_authorize(
    const unsigned char *script_hash, const unsigned char *args,
    unsigned long args_len) {
  (void)args;
  if (args_len > 64 || !script_hash_is_nonzero(script_hash)) {
    return 2;
  }

  return 1;
}
