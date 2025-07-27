# Stateless Light Clients for PoS Blockchains using Recursive STARKs

This repository contains the implementation for our paper on Stateless Light Clients for PoS Blockchains using Recursive STARKs. It leverages recursive STARKs to efficiently and verifiably track the Ethereum consensus.

The full details of the architecture, methodology, and results can be found in our research paper, which is included in this repository.

[**Read the paper here**](./paper/main.pdf)

## Repository Structure

-   `./cairo/`: Contains the Cairo programs for the recursive STARK verifier.
-   `./src/`: The core Rust implementation, including:
    -   `core`: The main light client logic, Beacon Chain client, and database interactions.
    -   `cli`: A command-line interface for generating proofs and managing the light client.
    -   `cairo_runner`: A helper crate to run Cairo programs.
-   `./benchmark/`: Scripts for performance benchmarking and generating diagrams from the results.
-   `./paper/`: The LaTeX source and compiled PDF of our research paper.

## Getting Started

### Prerequisites

You will need a working Rust toolchain and a Python 3 environment.

### Setup

1.  **Install dependencies and build the Cairo program:**

    This command will set up a Python virtual environment in `venv/`, install required packages, and compile the Cairo code.
    ```bash
    make setup
    make build-cairo
    ```

2.  **Configure Environment Variables:**

    The application requires an RPC endpoint for an Ethereum consensus layer client. You can copy the example file to create your own local configuration.
    ```bash
    cp .env.example .env.local
    ```
    Now, edit `.env.local` and add your RPC URL:
    ```
    RPC_URL_BEACON="https://your-rpc-url.com"
    ```

## Running the CLI

The CLI is the primary interface for interacting with the light client. It can generate proofs and fetch the on-chain data required to create them.

### Activate the Environment

Before running any CLI commands, you must activate the Python virtual environment, which is required by the Cairo runner:

```bash
source scripts/activate.sh
```

### Commands

The CLI supports two main operations: `prove` and `fetch`, each with subcommands for `genesis` and `recursive-epoch`. We recommend running with the `--release` flag (`-r`) for performance.

**1. Generate Genesis Proof**

This command creates the initial proof from a recent trusted epoch, which serves as the starting point for the light client.

```bash
cargo run -r --bin cli prove genesis
```

**2. Generate Recursive Update Proof**

After a genesis proof has been created, this command generates a new proof that updates the light client state from the last proven epoch to a more recent one.

```bash
cargo run -r --bin cli prove recursive-epoch
```
You can use the `--fast-forward` or `-f` flag to specify how many epochs to advance.

**3. Fetching Data**

For debugging purposes, you can use the `fetch` commands to download and inspect the data required for a proof without actually running the prover.

```bash
# Fetch data for genesis
cargo run -r --bin cli fetch genesis

# Fetch data for a recursive update
cargo run -r --bin cli fetch recursive-epoch
```

## Benchmarking & Visualization

The repository includes tools to benchmark the performance of the proof generation process and visualize the results. Make sure your Python virtual environment is activated before running these scripts.

### Running the Benchmark

The `benchmark/benchmark.py` script automates the process of generating recursive proofs over time and collecting performance metrics. It runs the prover at a set interval, parses the output, and saves the performance data to `benchmark/bankai_metrics.csv`.

To start the benchmark:
```bash
python benchmark/benchmark.py
```

### Generating Diagrams

After collecting data with the benchmark script, you can generate visualizations using `benchmark/diagrams.py`.

```bash
python benchmark/diagrams.py
```

This script reads `benchmark/bankai_metrics.csv` and creates several plots in the `benchmark/diagrams/` directory, such as:
- Proving time vs. Epoch
- Cairo step count vs. Epoch
- Proving time distribution

## Acknowledgements

This project would not have been possible without the incredible work and support of several teams in the ecosystem. We would like to extend our heartfelt thanks to:

-   **Herodotus** for supporting the underlying light client work, and providing access to [Altantic Prover](www.atlanticprover.com) for generating recursive proofs.
-   The **Garaga** team for their incredible work, enabling efficient elliptic curve and pairing operations in Cairo. 
-   **StarkWare** for continues support for this project.

We are immensely grateful for their contributions to the community and for building the tools that enabled this research.
