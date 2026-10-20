# Reggz

Reggz is a UCI chess engine written in Rust. It plays on Lichess as [Reggz](https://lichess.org/@/Reggz), and works with any UCI-compatible GUI, such as Cute Chess or En Croissant.

## Features

### Search

- Negamax with alpha-beta pruning
- Iterative deepening with aspiration windows
- Transposition table (256 MB) with Zobrist hashing
- Quiescence search with transposition table lookups
- Null-move pruning
- Late move reductions, with a re-search when a reduced move beats alpha
- Check extensions
- Move ordering: the transposition table move first, then promotions and captures by MVV-LVA
- Draw detection for repetition and the 50-move rule
- Mate scores adjusted by distance, so the engine prefers faster mates

### Evaluation

- Material and piece-square tables, with separate middlegame and endgame tables (values from [PeSTO](https://www.chessprogramming.org/PeSTO%27s_Evaluation_Function))

### UCI

- Time management based on the remaining clock time and increment
- Supports `go` with `wtime`, `btime`, `winc`, `binc`, `movetime`, `depth`, `infinite` and `ponder`
- Prints the principal variation, depth, node count and score (in centipawns or as mate in N)

Move generation uses the [shakmaty](https://crates.io/crates/shakmaty) crate.

## Building and running

Install Rust with [rustup](https://rustup.rs/), then:

```sh
cargo build --release
cargo run --release
```

The binary is written to `target/release/reggz`. Run the tests with `cargo test`.

## Testing

Each new version is tested against earlier versions with SPRT matches in [fastchess](https://github.com/Disservin/fastchess), and changes are kept only when they improve playing strength.

## License

[MIT](LICENSE)
