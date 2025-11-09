# sudachi_tantivy

[Sudachi](https://github.com/WorksApplications/sudachi.rs) tokenizer for [Tantivy](https://github.com/quickwit-oss/tantivy).

## Test

Download a dictionary file.

```sh
curl -LO --output-dir tests/resources/ https://github.com/WorksApplications/SudachiDict/releases/download/v20251022/sudachi-dictionary-20251022-core.zip
unzip -d tests/resources/ tests/resources/sudachi-dictionary-20251022-core.zip
```

Run `cargo test` with the dictionary path specified in the environment variable `SUDACHI_DICT_PATH`.

```sh
SUDACHI_DICT_PATH=tests/resources/sudachi-dictionary-20251022/system_core.dic cargo test
```
