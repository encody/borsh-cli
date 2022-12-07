# Borsh CLI

Command line utility for basic [Borsh](https://borsh.io/)-serialized data manipulations.

## Install

```txt
$ cargo install borsh-cli
```

## Usage

```text
Command-line utility for manipulating Borsh-serialized data

Note: Does not play particularly nicely with `HashMap<_, _>` types.

Usage: borsh[EXE] <COMMAND>

Commands:
  pack
          Serialize the input as a simple binary blob with Borsh headers
  unpack
          Deserialize the input as a simple binary blob with Borsh headers
  encode
          Convert JSON to Borsh
  decode
          Decode Borsh input to JSON
  extract
          Extracts the Borsh schema header
  strip
          Removes the Borsh schema header
  help
          Print this message or the help of the given subcommand(s)

Options:
  -h, --help
          Print help information (use `-h` for a summary)

  -V, --version
          Print version information
```

Generally, every sub-command will read from STDIN and output to STDOUT unless the `-i`/`--input` flag or `-o`/`--output` flags are specified, respectively.

## Examples

### Pack

```text
$ echo 'hello' | borsh pack | base64
BgAAAGhlbGxvCg==
```

### Encode

#### With schema

Recommended for most use-cases, and for highly-structured data.

```text
$ cat schema.borshschema | base64
BQAAAEZpcnN0CAAAAAUAAABGaXJzdAQABAAAAAEAAABhDwAAAFR1cGxlPHUzMiwgdTY0PgEAAABi
BgAAAHN0cmluZwEAAABjBgAAAFNlY29uZAEAAABlCwAAAFZlYzxzdHJpbmc+BgAAAFNlY29uZAQA
BQAAAAEAAABhBQAAAFRoaXJkAQAAAGIFAAAAVGhpcmQBAAAAYwUAAABUaGlyZAEAAABkAwAAAHUz
MgEAAABlAwAAAHUzMgUAAABUaGlyZAMDAAAABQAAAEFscGhhCgAAAFRoaXJkQWxwaGEEAAAAQmV0
YQkAAABUaGlyZEJldGEFAAAAR2FtbWEKAAAAVGhpcmRHYW1tYQoAAABUaGlyZEFscGhhBAABAAAA
BQAAAGZpZWxkAwAAAHUzMgkAAABUaGlyZEJldGEEAQEAAAADAAAAdTMyCgAAAFRoaXJkR2FtbWEE
Ag8AAABUdXBsZTx1MzIsIHU2ND4CAgAAAAMAAAB1MzIDAAAAdTY0CwAAAFZlYzxzdHJpbmc+AQYA
AABzdHJpbmc=

$ cat data.json
{
  "a": [32, 64],
  "b": "String",
  "c": {
    "a": { "Alpha": { "field": 1 } },
    "b": { "Beta": 1 },
    "c": "Gamma",
    "d": 2,
    "e": 3
  },
  "e": ["a", "b", "c"]
}

$ borsh encode -i data.json -s schema.borshschema -o data.borsh
$ cat data.borsh | base64
BQAAAEZpcnN0CAAAAAUAAABGaXJzdAQABAAAAAEAAABhDwAAAFR1cGxlPHUzMiwgdTY0PgEAAABi
BgAAAHN0cmluZwEAAABjBgAAAFNlY29uZAEAAABlCwAAAFZlYzxzdHJpbmc+BgAAAFNlY29uZAQA
BQAAAAEAAABhBQAAAFRoaXJkAQAAAGIFAAAAVGhpcmQBAAAAYwUAAABUaGlyZAEAAABkAwAAAHUz
MgEAAABlAwAAAHUzMgUAAABUaGlyZAMDAAAABQAAAEFscGhhCgAAAFRoaXJkQWxwaGEEAAAAQmV0
YQkAAABUaGlyZEJldGEFAAAAR2FtbWEKAAAAVGhpcmRHYW1tYQoAAABUaGlyZEFscGhhBAABAAAA
BQAAAGZpZWxkAwAAAHUzMgkAAABUaGlyZEJldGEEAQEAAAADAAAAdTMyCgAAAFRoaXJkR2FtbWEE
Ag8AAABUdXBsZTx1MzIsIHU2ND4CAgAAAAMAAAB1MzIDAAAAdTY0CwAAAFZlYzxzdHJpbmc+AQYA
AABzdHJpbmcgAAAAQAAAAAAAAAAGAAAAU3RyaW5nAAEAAAABAQAAAAICAAAAAwAAAAMAAAABAAAA
YQEAAABiAQAAAGM=

$ borsh decode -i data.borsh
{"e":["a","b","c"],"b":"String","a":[32,64],"c":{"e":3,"b":{"Beta":[1]},"a":{"Alpha":{"field":1}},"c":{"Gamma":[]},"d":2}}
```

#### Without schema

Not recommended for highly-structured data.

```text
$ cat data.json
{
  "definitely_pi": 2.718281828459045,
  "trustworthy": false
}

$ borsh encode -i data.json -o data.borsh
$ cat data.borsh | base64
aVcUiwq/BUAA
```

Note: Fields are encoded in the order of their appearance. Thus, the encoding of `{"a":1,"b":2}` is different from that of `{"b":2,"a":1}`.

### Decode

Requires that the input file contains Borsh schema headers.

### Strip

Removes the schema headers from some Borsh data and returns the remaining data.

### Extract

Returns just the schema headers from some Borsh data.

## FAQ

### How to generate Borsh schema headers for my data?

The `borsh` Rust crate contains a macro for automatically generating Borsh schema headers for your data:

```rust
use borsh::{BorshSchema, BorshSerialize};

#[derive(BorshSerialize, BorshSchema)]
struct MyStruct { /* ... */ }

fn serialize(data: MyStruct) {
  let serialized_with_schema: Vec<u8> = borsh::try_to_vec_with_schema(&data).unwrap();

  // ...
}
```

## Authors

- Jacob Lindahl [@sudo_build](https://twitter.com/sudo_build)
