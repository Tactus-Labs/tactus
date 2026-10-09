//! Runs the A1 fee-ratio × signing-delay sweep of Experiment A, test group 1
//! (design §7.1): the fee-competition and dead-on-arrival variables of
//! OrderingHead admission under a dominant builder, printed as a report.
//! The A3′ churn sensitivity table (analytic, design §3) is printed alongside.

use tactus_experiment_a::sim::{run_a1, A1Params};
use tactus_experiment_a::{survival_probability, Candidate, LoadModel};

fn main() {
    println!("Tactus Experiment A — arm {:?} (OrderingHead admission, simulation)", Candidate::A1AtomicHead);
    println!("2,000 blocks · dominant builder (period 2, fee 1.0) · 5 wallet users · retries 8\n");

    println!("=== Fee-ratio × signing-delay sweep (design §7.1) ===");
    println!(
        "{:>8} | {:>13} | {:>8} | {:>8} | {:>8} | {:>10} | {:>8}",
        "delay", "fee×builder", "succ%", "p50 dly", "p95 dly", "DOA-bcast%", "bch/blk"
    );
    for delay in [1u64, 2, 3] {
        for multiplier in [1.0_f64, 2.0, 5.0, 10.0] {
            let p = A1Params {
                signing_delay: delay,
                user_fee_multiplier: multiplier,
                ..A1Params::default()
            };
            let (s, m) = run_a1(&p);
            println!(
                "{:>8} | {:>13} | {:>7.1} | {:>8.1} | {:>8.1} | {:>10.1} | {:>8.2}",
                delay,
                format!("×{multiplier}"),
                s.success_rate() * 100.0,
                m.priority_admission_delay_blocks_p50,
                m.priority_admission_delay_blocks_p95,
                m.head_stale_before_broadcast_rate * 100.0,
                m.builder_batches_per_block,
            );
        }
    }

    println!("\n=== A3′ survival sensitivity (analytic; design §3) ===");
    println!("L1 fixed per-lane λ=0.05/s | L2 fixed aggregate Λ=0.8/s | Δt=15s\n");
    println!("  {:>6} | {:>12} | {:>12}", "lanes", "L1 e^(-KλΔt)", "L2 e^(-ΛΔt)");
    for k in [1_usize, 2, 4, 8, 16] {
        let l1 = survival_probability(15.0, &vec![0.05; k], LoadModel::FixedPerLane);
        let l2 = survival_probability(15.0, &vec![0.8 / k as f64; k], LoadModel::FixedAggregate);
        println!("  {k:>6} | {l1:>12.4} | {l2:>12.4}");
    }
    println!("  L3 adversarial: no analytic form — measure it (design §3)");
}
