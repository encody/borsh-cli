# Borsh CLI

Command line utility for basic [Borsh](https://borsh.io/)-serialized data manipulations.

## Usage

```text
Usage: borsh[EXE] <COMMAND>

Commands:
  pack    Serialize the input as a simple binary blob with Borsh headers
  unpack  Deserialize the input as a simple binary blob with Borsh headers
  encode  Convert JSON to Borsh
  decode  NOT IMPLEMENTED -- Decode Borsh input to JSON
  help    Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help information
  -V, --version  Print version information
```

Generally, every sub-command will read from STDIN and output to STDOUT unless the `-i`/`--input` flag or `-o`/`--output` flags are specified, respectively.

## Examples

### Pack

```text
$ echo 'hello' | borsh pack | base64
BgAAAGhlbGxvCg==
```

### JSON

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

### Decoding

Decoding Borsh data is not yet supported. It's not quite as simple as just "reversing the encoding process" as it requires specifying a schema for the data being decoded. I do plan to implement it, but I just haven't had the time yet. However, generally, I think this tool may still be useful for sending Borsh-serialized data to applications that accept it.

## Authors

- Jacob Lindahl [@sudo_build](https://twitter.com/sudo_build)
