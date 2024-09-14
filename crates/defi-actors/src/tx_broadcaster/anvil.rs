use std::marker::PhantomData;

use alloy_eips::BlockNumberOrTag;
use alloy_network::{Ethereum, Network};
use alloy_provider::Provider;
use alloy_rpc_types::BlockTransactions;
use alloy_transport::Transport;
use eyre::Result;
use log::{error, info};
use tokio::sync::broadcast::error::RecvError;
use tokio::sync::broadcast::Receiver;

use debug_provider::AnvilProviderExt;
use defi_blockchain::Blockchain;
use defi_events::{MessageTxCompose, TxCompose, TxComposeData};
use loom_actors::{Actor, ActorResult, Broadcaster, Consumer, WorkerResult};
use loom_actors_macros::{Accessor, Consumer};

async fn broadcast_task<P, T, N>(client: P, request: TxComposeData) -> Result<()>
where
    N: Network,
    T: Transport + Clone,
    P: Provider<T, N> + AnvilProviderExt<T, N> + Clone + Send + Sync + 'static,
{
    info!("Hardhat broadcast request received : {}", request.origin.unwrap_or("UNKNOWN_ORIGIN".to_string()));
    //let snap = client.dev_rpc().snapshot().await?;
    //info!("Hardhat snapshot created {snap}");

    for tx_rlp in request.rlp_bundle.unwrap_or_default().iter() {
        let tx_bytes = tx_rlp.clone().unwrap().clone();

        //let envelope = TxEnvelope::decode_2718(&mut tx_bytes.as_ref())?;
        //debug!("sending tx to anvil: {} {:?}", tx_bytes.len(), envelope);

        match client.send_raw_transaction(&tx_bytes).await {
            Err(e) => error!("send_raw_transaction error : {e}"),
            Ok(_) => {
                info!("send_raw_transaction error : Hardhat transaction broadcast successfully",);
            }
        }
    }

    Ok(())
}

async fn anvil_broadcaster_worker<P, T>(client: P, bundle_rx: Broadcaster<MessageTxCompose>) -> WorkerResult
where
    T: Transport + Clone,
    P: Provider<T, Ethereum> + AnvilProviderExt<T, Ethereum> + Send + Sync + Clone + 'static,
{
    let mut bundle_rx: Receiver<MessageTxCompose> = bundle_rx.subscribe().await;

    loop {
        tokio::select! {
            msg = bundle_rx.recv() => {
                let broadcast_msg : Result<MessageTxCompose, RecvError> = msg;
                match broadcast_msg {
                    Ok(compose_request) => {
                        if let TxCompose::Broadcast(broadcast_request) = compose_request.inner {
                            info!("Broadcasting to hardhat:" );
                            let snap_shot = client.snapshot().await?;
                            client.set_automine(false).await?;
                            match broadcast_task(client.clone(), broadcast_request).await{
                                Err(e)=>error!("{e}"),
                                Ok(_)=>info!("Hardhat broadcast successful")
                            }
                            client.mine().await?;

                            let block = client.get_block_by_number(BlockNumberOrTag::Latest, false).await?.unwrap_or_default();
                            if let BlockTransactions::Hashes(hashes) = block.transactions {
                                for tx_hash in hashes {
                                    let reciept = client.get_transaction_receipt(tx_hash).await?.unwrap();
                                    info!("Block : {} Mined: {} hash:  {} gas : {}", reciept.block_number.unwrap_or_default(), reciept.status(), tx_hash, reciept.gas_used, );
                                }
                            }
                            client.revert(snap_shot).await?;
                        }
                    }
                    Err(e)=>{
                        error!("{}", e)
                    }
                }
            }
        }
    }
}

#[derive(Accessor, Consumer)]
pub struct AnvilBroadcastActor<P, T> {
    client: P,
    #[consumer]
    tx_compose_rx: Option<Broadcaster<MessageTxCompose>>,
    _t: PhantomData<T>,
}

impl<P, T> AnvilBroadcastActor<P, T>
where
    T: Transport + Clone,
    P: Provider<T, Ethereum> + AnvilProviderExt<T, Ethereum> + Send + Sync + Clone + 'static,
{
    pub fn new(client: P) -> AnvilBroadcastActor<P, T> {
        Self { client, tx_compose_rx: None, _t: PhantomData }
    }

    pub fn on_bc(self, bc: &Blockchain) -> Self {
        Self { tx_compose_rx: Some(bc.compose_channel()), ..self }
    }
}

impl<P, T> Actor for AnvilBroadcastActor<P, T>
where
    T: Transport + Clone,
    P: Provider<T, Ethereum> + AnvilProviderExt<T, Ethereum> + Send + Sync + Clone + 'static,
{
    fn start(&self) -> ActorResult {
        let task = tokio::task::Builder::new()
            .name(self.name())
            .spawn(anvil_broadcaster_worker(self.client.clone(), self.tx_compose_rx.clone().unwrap()))?;
        Ok(vec![task])
    }

    fn name(&self) -> &'static str {
        "AnvilBroadcastActor"
    }
}
