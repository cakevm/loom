use std::convert::TryFrom;
use std::sync::Arc;

use alloy_primitives::{B256, TxHash, U64};
use alloy_provider::Provider;
use alloy_signer::{Signature, Signer, SignerSync};
use alloy_signer_wallet::{LocalWallet, Wallet};
use eyre::{ErrReport, eyre, Result};
use log::{debug, error, info};
use rand::thread_rng;
use url::Url;

use crate::client::{BundleRequest, BundleTransaction, FlashbotsMiddleware, FlashbotsMiddlewareError, SimulatedBundle};

pub struct FlashbotsClient<P> {
    pub flashbots_middleware: FlashbotsMiddleware<P>,
    pub name: String,
}

impl<P: Provider + Send + Sync + Clone + 'static> FlashbotsClient<P> {
    pub fn new(provider: P, url: &str) -> Self {
        let flashbots_middleware = Self::create_flashbots_middleware(provider, url);

        let name = url.to_string();

        FlashbotsClient {
            flashbots_middleware,
            name,
        }
    }

    pub fn new_no_sign(provider: P, url: &str) -> Self {
        let flashbots_client = FlashbotsClient::create_flashbots_no_signer_middleware(provider, url);

        let name = url.to_string();

        FlashbotsClient {
            flashbots_middleware: flashbots_client,
            name,
        }
    }


    fn create_flashbots_middleware(provider: P, url: &str) -> FlashbotsMiddleware<P> {
        let flashbots: FlashbotsMiddleware<P> = FlashbotsMiddleware::new(
            Url::parse(url).unwrap(),
            provider,
        );

        flashbots
    }

    fn create_flashbots_no_signer_middleware(provider: P, url: &str) -> FlashbotsMiddleware<P> {
        let flashbots: FlashbotsMiddleware<P> = FlashbotsMiddleware::new_no_signer(
            Url::parse(url).unwrap(),
            provider,
        );
        flashbots
    }

    pub async fn call_bundle(&self, request: &BundleRequest) -> Result<SimulatedBundle>
    {
        match self.flashbots_middleware.simulate_local_bundle(request).await {
            Ok(x) => Ok(x),
            Err(e) => {
                error!("{}", e);
                Err(eyre!("FLASHBOTS LOCAL ERROR"))

                /*match e {
                    FlashbotsMiddlewareError::MiddlewareError(e) => {
                        error!("{:?}", e);
                        Err(eyre!("FLASHBOTS MIDDLEWARE ERROR"))
                    }
                    FlashbotsMiddlewareError::RelayError(e) => {
                        Err(eyre!("FLASHBOTS RELAY ERROR"))
                    }
                    FlashbotsMiddlewareError::MissingParameters => {
                        Err(eyre!("FLASHBOTS MISSING PARAMETERS ERROR"))
                    }
                }*/
            }
        }
    }

    pub async fn send_bundle(&self, request: &BundleRequest) -> Result<()> {
        match self.flashbots_middleware.send_bundle(request).await {
            Ok(resp) => {
                info!("Bundle sent to : {}", self.name );
                Ok(())
            }
            Err(error) => {
                match error {
                    FlashbotsMiddlewareError::MissingParameters => {
                        error!("{} : Missing paramter", self.name);
                        Err(ErrReport::msg("FLASHBOTS_MISSING_PARAMETER"))
                    }
                    FlashbotsMiddlewareError::RelayError(x) => {
                        error!("{} {}", self.name, x.to_string() );
                        Err(ErrReport::msg("FLASHBOTS_RELAY_ERROR"))
                    }
                    FlashbotsMiddlewareError::MiddlewareError(x) => {
                        error!("{} {}", self.name, x.to_string() );
                        Err(ErrReport::msg("FLASHBOTS_MIDDLEWARE_ERROR"))
                    }
                }
            }
        }
    }
}

pub struct Flashbots<P>
{
    simulation_client: FlashbotsClient<P>,
    clients: Vec<Arc<FlashbotsClient<P>>>,
}


impl<P: Provider + Send + Sync + Clone + 'static> Flashbots<P> {
    pub fn new(provider: P, simulation_endpoint: &str) -> Self
    {
        let flashbots = FlashbotsClient::new(provider.clone(), "https://relay.flashbots.net");
        let builder0x69 = FlashbotsClient::new_no_sign(provider.clone(), "https://builder0x69.io");
        let eden = FlashbotsClient::new(provider.clone(), "https://api.edennetwork.io/v1/bundle");
        let eth_builder = FlashbotsClient::new_no_sign(provider.clone(), "https://eth-builder.com");
        let beaverbuild = FlashbotsClient::new_no_sign(provider.clone(), "https://rpc.beaverbuild.org/");
        let secureapi = FlashbotsClient::new_no_sign(provider.clone(), "https://api.securerpc.com/v1");
        let rsync = FlashbotsClient::new_no_sign(provider.clone(), "https://rsync-builder.xyz");
        //let blocknative = FlashbotsClient::new(provider.clone(), "https://api.blocknative.com/v1/auction");
        let buildai = FlashbotsClient::new_no_sign(provider.clone(), "https://BuildAI.net");
        let payloadde = FlashbotsClient::new_no_sign(provider.clone(), "https://rpc.payload.de");
        let titan = FlashbotsClient::new(provider.clone(), "https://rpc.titanbuilder.xyz");
        let fibio = FlashbotsClient::new(provider.clone(), "https://rpc.f1b.io");
        let loki = FlashbotsClient::new(provider.clone(), "https://rpc.lokibuilder.xyz");
        let ibuilder = FlashbotsClient::new(provider.clone(), "https://rpc.ibuilder.xyz");
        let jetbuilder = FlashbotsClient::new(provider.clone(), "https://rpc.jetbldr.xyz");
        let penguinbuilder = FlashbotsClient::new(provider.clone(), "https://rpc.penguinbuild.org");
        let gambitbuilder = FlashbotsClient::new(provider.clone(), "https://builder.gmbit.co/rpc");


        let clients_vec = vec![flashbots, /* builder0x69,*/ titan, fibio, eden, eth_builder, beaverbuild, secureapi, rsync, /*blocknative,*/ buildai, payloadde, loki, ibuilder, jetbuilder, penguinbuilder, gambitbuilder];

        Flashbots {
            clients: clients_vec.into_iter().map(|x| Arc::new(x)).collect(),
            simulation_client: FlashbotsClient::new(provider.clone(), simulation_endpoint),
        }
    }


    pub async fn simulate_txes<T>(&self, txs: Vec<T>, block_number: u64, access_list_request: Option<Vec<TxHash>>) -> Result<SimulatedBundle>
        where BundleTransaction: std::convert::From<T>
    {
        let mut bundle = BundleRequest::new()
            .set_block(U64::from(block_number + 1))
            .set_simulation_block(U64::from(block_number))
            .set_access_list_hashes(access_list_request);

        for t in txs.into_iter() {
            bundle = bundle.push_transaction(t);
        }

        self.simulation_client.call_bundle(&bundle).await
    }


    pub async fn broadcast_txes<T>(&self, txs: Vec<T>, block: u64) -> Result<()>
        where BundleTransaction: std::convert::From<T>
    {
        let mut bundle = BundleRequest::new().set_block(U64::from(block));

        for t in txs.into_iter() {
            bundle = bundle.push_transaction(t);
        }

        let bundle_arc = Arc::new(bundle);

        for client in self.clients.iter() {
            let client_clone = client.clone();
            let bundle_arc_clone = bundle_arc.clone();
            tokio::task::spawn(async move {
                debug!("Sending bundle to {}", client_clone.name);
                let bundle_result = client_clone.send_bundle(bundle_arc_clone.as_ref()).await;
                match bundle_result {
                    Ok(x) => {
                        info!("Flashbots bundle broadcast successfully {}", client_clone.name);
                    }
                    Err(x) => {
                        error!("Broadcasting error to {} : {}", client_clone.name, x.to_string());
                    }
                }
            });
        };


        Ok(())
    }
}


#[cfg(test)]
mod tests {
    use std::time::Duration;

    use env_logger::Env;
    use ethers::prelude::{H256, Provider, Ws};

    use super::*;

    #[tokio::test]
    async fn test_broadcast_txs() {
        std::env::set_var("RUST_LOG", "debug");
        std::env::set_var("RUST_BACKTRACE", "1");
        env_logger::init_from_env(Env::default().default_filter_or("debug"));

        const WS_URL: &str = "wss://eth-mainnet.alchemyapi.io/v2/v5AzlmuG3aN7FFF0FkXhF396y9ISRIsu";

        let provider = Provider::<Ws>::connect_with_reconnects(WS_URL, 10).await.unwrap();
        let client = Arc::new(provider);

        let flashbots_client = Flashbots::new(client.clone());

        let tx = client.get_transaction(H256::from_slice(
            &hex::decode("6efe8576460f0493f66bf9565f2ed906e3db2eebea88b179a027c72c2cc30146").unwrap())).await.unwrap().unwrap();

        let rlp_encoded = tx.rlp();


        flashbots_client.broadcast_txes(vec![tx.clone()], tx.block_number.unwrap()).await;

        tokio::time::sleep(Duration::from_secs(10)).await;
    }
}