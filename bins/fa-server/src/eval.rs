//! `fa-server eval`: replays the labelled sample through the pipeline. Read-only; never run it
//! against real data from a script or a test.

use anyhow::{Result, anyhow, bail};
use fa_config::ProcessConfig;
use fa_domain::{DecisionPointKey, HouseholdId};
use fa_eval::{EvalError, EvalOpts};
use fa_store::{HouseholdRepo, Store};

use crate::wiring;

/// The given household, or the only one in the database (ops.rs keeps its copy private).
async fn household_or_only(store: &Store, given: Option<HouseholdId>) -> Result<HouseholdId> {
    if let Some(h) = given {
        return Ok(h);
    }
    let mut tx = store.tx_system().await?;
    let all = HouseholdRepo::list(&mut tx).await?;
    tx.rollback().await?;
    match all.as_slice() {
        [] => Err(anyhow!(
            "no household yet; open the app in a browser to run setup"
        )),
        [h] => Ok(h.value.id),
        _ => bail!("several households exist; pass --household <id>"),
    }
}

pub async fn run(
    cfg: &ProcessConfig,
    store: &Store,
    household: Option<HouseholdId>,
    dp: DecisionPointKey,
    opts: EvalOpts,
    json: bool,
) -> Result<()> {
    let hid = household_or_only(store, household).await?;
    let llm = if opts.with_llm {
        Some(wiring::llm_provider(cfg, store)?)
    } else {
        None
    };
    let mut tx = store.tx_for(hid).await?;
    let res = fa_eval::run(&mut tx, hid, dp, opts, llm.as_deref()).await;
    drop(tx);
    match res {
        Ok(report) => {
            if json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                print!("{}", report.to_table());
            }
            Ok(())
        }
        Err(EvalError::NotConfigured(msg)) => {
            eprintln!("error: {msg}; enable one in Settings -> AI, or run without --with-llm");
            std::process::exit(2);
        }
        Err(e) => Err(e.into()),
    }
}
