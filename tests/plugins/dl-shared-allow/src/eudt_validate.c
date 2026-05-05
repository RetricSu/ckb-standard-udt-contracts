typedef unsigned long size_t;

static int matches_require_mint_checked(const unsigned char *data, size_t len) {
  const unsigned char expected[] = "require_mint_checked";
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

static int matches_require_mint_none(const unsigned char *data, size_t len) {
  const unsigned char expected[] = "require_mint_none";
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

__attribute__((visibility("default"))) int eudt_validate(
    const unsigned char *script_hash, unsigned char op_type,
    unsigned char ext_index, const unsigned char *ext_data_ptr,
    unsigned long ext_data_len, unsigned char mint_authority_checked) {
  (void)script_hash;
  (void)op_type;
  (void)ext_index;

  if (matches_require_mint_checked(ext_data_ptr, ext_data_len) &&
      mint_authority_checked != 1) {
    return 1;
  }
  if (matches_require_mint_none(ext_data_ptr, ext_data_len) &&
      mint_authority_checked != 2) {
    return 2;
  }

  return 0;
}
