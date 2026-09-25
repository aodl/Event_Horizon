use std::collections::{BTreeMap, BTreeSet};

use candid::Principal;

use crate::{
    account::default_account_identifier,
    clients::{historian, ledger, subscriber},
    config, logging,
    memo::{parse_subscription_memo, SubscriptionDeclaration},
    pricing, state,
    subscription::Subscription,
};

#[derive(Default)]
struct MatchState {
    global_matched: bool,
    matched_subaccounts: BTreeSet<u8>,
}

fn archived_prefix_end(ranges: &[ledger::ArchivedBlocksRange], start: u64, boundary: u64) -> u64 {
    let mut end = start;
    loop {
        let mut advanced = end;
        for range in ranges {
            let range_end = range.start.saturating_add(range.length).min(boundary);
            if range.start <= end && end < range_end {
                advanced = advanced.max(range_end);
            }
        }
        if advanced == end {
            return end;
        }
        end = advanced;
    }
}
fn account_id(bytes: &[u8]) -> Option<[u8; 32]> {
    let mut out = [0u8; 32];
    if bytes.len() != out.len() {
        return None;
    }
    out.copy_from_slice(bytes);
    Some(out)
}

async fn process_transfer(
    from: &[u8],
    to: &[u8],
    amount_e8s: u64,
    icrc1_memo: Option<&[u8]>,
    matches: &mut BTreeMap<Principal, MatchState>,
) {
    let runtime = config::runtime();
    let self_id = ic_cdk::api::canister_self();
    let faucet_account = default_account_identifier(runtime.faucet_canister);
    let self_account = default_account_identifier(self_id);

    let from_id = account_id(from);
    let to_id = account_id(to);

    if from_id == Some(faucet_account) && to_id == Some(self_account) {
        if let Some(memo) = icrc1_memo {
            if let Ok(declaration) = parse_subscription_memo(memo) {
                let pricing_class = match &declaration {
                    SubscriptionDeclaration::Global { .. } => pricing::PricingClass::Global,
                    SubscriptionDeclaration::Account { .. } => pricing::PricingClass::Account,
                    SubscriptionDeclaration::Range { .. } => pricing::PricingClass::Range,
                };
                let Some(required_e8s) = pricing::current_admission_e8s(pricing_class) else {
                    return;
                };
                match historian::route_is_admitted(
                    runtime.historian_canister,
                    self_id,
                    memo.to_vec(),
                    required_e8s,
                )
                .await
                {
                    Ok(true) => {
                        match declaration {
                            SubscriptionDeclaration::Global { subscriber } => {
                                state::put_global_subscriber(subscriber);
                            }
                            account @ SubscriptionDeclaration::Account { .. } => {
                                state::put_subscription(Subscription::from(account));
                            }
                            SubscriptionDeclaration::Range {
                                subscriber,
                                start_subaccount,
                                end_subaccount,
                                minimum_e8s,
                            } => {
                                // Expansion is protocol-bounded to 256 stable-map merges and
                                // keeps the per-transfer path as one account-identifier lookup.
                                for numbered_subaccount in start_subaccount..=end_subaccount {
                                    state::put_subscription(Subscription {
                                        subscriber,
                                        numbered_subaccount,
                                        minimum_e8s,
                                    });
                                }
                            }
                        }
                        logging::historian_recovered();
                    }
                    Ok(false) => logging::historian_recovered(),
                    Err(error) if error == "reserve_protection" => {}
                    Err(error) => logging::historian_failure(&error),
                }
            }
        }
    }

    if let Some(destination) = to_id {
        if let Some(subscription) = state::get_subscription(destination) {
            if subscription.matches(amount_e8s) {
                matches
                    .entry(subscription.subscriber)
                    .or_default()
                    .matched_subaccounts
                    .insert(subscription.numbered_subaccount);
            }
        }
    }
}

async fn process_page(
    response: ledger::QueryBlocksResponse,
    mut cursor: u64,
    boundary: u64,
    matches: &mut BTreeMap<Principal, MatchState>,
    processed_any_transactions: &mut bool,
) -> Result<u64, String> {
    if cursor >= boundary {
        return Ok(cursor);
    }

    let original_cursor = cursor;
    let archived_end = archived_prefix_end(&response.archived_blocks, cursor, boundary);
    if archived_end > cursor {
        logging::history_gap(cursor, archived_end);
        cursor = archived_end;
        state::modify_metadata(|meta| meta.next_block = cursor);
    }

    if response.blocks.is_empty() {
        if cursor > original_cursor || cursor >= boundary {
            return Ok(cursor);
        }
        return Err("query_blocks returned no live progress for the requested cursor".to_string());
    }

    if response.first_block_index > cursor {
        return Err(format!(
            "query_blocks returned unexplained hole cursor={cursor} first_block_index={}",
            response.first_block_index
        ));
    }

    let mut processed_end = cursor;
    for (offset, block) in response.blocks.into_iter().enumerate() {
        let index = response.first_block_index.saturating_add(offset as u64);
        if index < cursor {
            continue;
        }
        if index >= boundary {
            break;
        }
        if index != processed_end {
            return Err(format!(
                "non-contiguous block response expected={processed_end} got={index}"
            ));
        }

        *processed_any_transactions = true;

        if let Some(ledger::Operation::Transfer {
            from, to, amount, ..
        }) = &block.transaction.operation
        {
            process_transfer(
                from,
                to,
                amount.e8s,
                block.transaction.icrc1_memo.as_deref(),
                matches,
            )
            .await;
        }
        processed_end = index.saturating_add(1);
    }

    if processed_end == cursor {
        return Err("query_blocks page contained no processable blocks".to_string());
    }
    state::modify_metadata(|meta| meta.next_block = processed_end);
    logging::history_live_progress();
    Ok(processed_end)
}

pub async fn run_poll() {
    let runtime = config::runtime();
    let mut meta = state::read_metadata();

    if !meta.bootstrapped {
        match ledger::query_blocks(runtime.icp_ledger, 0, 0).await {
            Ok(response) => {
                meta.bootstrapped = true;
                meta.next_block = response.chain_length;
                state::write_metadata(meta);
                logging::ledger_recovered();
            }
            Err(error) if error == "reserve_protection" => {}
            Err(error) => logging::ledger_failure(&error),
        }
        return;
    }

    let mut cursor = meta.next_block;
    let first =
        match ledger::query_blocks(runtime.icp_ledger, cursor, config::LEDGER_PAGE_SIZE).await {
            Ok(response) => {
                logging::ledger_recovered();
                response
            }
            Err(error) if error == "reserve_protection" => return,
            Err(error) => {
                logging::ledger_failure(&error);
                return;
            }
        };
    let boundary = first.chain_length;
    if cursor >= boundary {
        return;
    }

    let mut matches: BTreeMap<Principal, MatchState> = BTreeMap::new();
    let mut processed_any_transactions = false;
    let mut next_response = Some(first);
    while cursor < boundary {
        let response = if let Some(response) = next_response.take() {
            response
        } else {
            let length = config::LEDGER_PAGE_SIZE.min(boundary.saturating_sub(cursor));
            match ledger::query_blocks(runtime.icp_ledger, cursor, length).await {
                Ok(response) => {
                    logging::ledger_recovered();
                    response
                }
                Err(error) if error == "reserve_protection" => return,
                Err(error) => {
                    logging::ledger_failure(&error);
                    return;
                }
            }
        };
        match process_page(
            response,
            cursor,
            boundary,
            &mut matches,
            &mut processed_any_transactions,
        )
        .await
        {
            Ok(new_cursor) if new_cursor > cursor => cursor = new_cursor,
            Ok(_) => return,
            Err(error) => {
                logging::ledger_failure(&error);
                return;
            }
        }
    }

    if processed_any_transactions {
        for principal in state::global_subscribers() {
            matches.entry(principal).or_default().global_matched = true;
        }
    }

    for (principal, matched) in matches {
        if !matched.matched_subaccounts.is_empty() {
            let _ = subscriber::poke(principal, matched.matched_subaccounts.into_iter().collect());
        } else if matched.global_matched {
            let _ = subscriber::poke(principal, vec![]);
        }
    }
}
