# Hokkaido

Advanced Data Structures Lib written in Rust

## Gojo

Gojo is a implementation of a Partial Persistence Red Black Tree

![gojo](https://qph.cf2.quoracdn.net/main-qimg-afef71370d28d3b966ad766ff8e5407d)

### Usage

You can use `Gojo` as part of your library or use it as a `binary`.

On binary mode you can use as input a file or stdin and as output stdout or a file.

```bash
echo "inc 1\ninc 2\ninc 3\ninc 4\ninc 5\nimp 10" | cargo run --bin gojo
```

```bash
cargo run --bin gojo -- -i test1.txt > result.txt
```

```bash
cargo run --bin gojo -- -i test1.txt
```

```bash
cargo run --bin gojo -- -i test1.txt -o result_test1.txt
```

## Konan

Konan is a naive implementation of a Search-optimized Packed Memory Array

![konan](https://static1.cbrimages.com/wordpress/wp-content/uploads/2017/05/Naruto-Konan.jpg)

### Usage

You can use `Konan` as part of your library or use it as a `binary`.

On binary mode you can use as input a file or stdin and as output stdout or a file.

```bash
echo "inc 1\ninc 2\ninc 3\ninc 4\ninc 5\nimp" | cargo run --bin konan
```

```bash
cargo run --bin konan -- -i test1.txt > result.txt
```

```bash
cargo run --bin konan -- -i test1.txt
```

```bash
cargo run --bin konan -- -i test1.txt -o result_test1.txt
```
