typedef unsigned long size_t;

static int matches_literal(const unsigned char *data, size_t len,
                           const unsigned char *expected,
                           size_t expected_len) {
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

static int script_hash_is_nonzero(const unsigned char *script_hash) {
  size_t index = 0;
  for (index = 0; index < 32; index++) {
    if (script_hash[index] != 0) {
      return 1;
    }
  }
  return 0;
}

#define MATCHES(data, len, literal)                                            \
  matches_literal((data), (len), (const unsigned char *)(literal),             \
                  sizeof(literal) - 1)

__attribute__((visibility("default"))) int udt_authorize(
    const unsigned char *script_hash, const unsigned char *args,
    unsigned long args_len) {
  if (MATCHES(args, args_len, "allow")) {
    return 0;
  }
  if (MATCHES(args, args_len, "require_hash")) {
    return script_hash_is_nonzero(script_hash) ? 0 : 1;
  }
  return 1;
}
