use alloy::network::Ethereum;
use alloy::primitives::U64;
use alloy::providers::{Provider, ProviderCall, ProviderLayer, RootProvider};
use alloy::rpc::client::NoParams;
use alloy::transports::{Transport, TransportErrorKind};
use reth_rpc_api::eth::core::FullEthApiServer;
use std::future::ready;
use std::marker::PhantomData;

pub struct RethNodeLayer<EthApi: FullEthApiServer> {
    eth_api: EthApi,
}

/// Initialize the `RethDBLayer` with the path to the reth datadir.
impl<EthApi> RethNodeLayer<EthApi>
where
    EthApi: FullEthApiServer,
{
    pub const fn new(eth_api: EthApi) -> Self {
        Self { eth_api }
    }

    pub(crate) const fn eth_api(&self) -> &EthApi {
        &self.eth_api
    }
}

impl<P, T, EthApi> ProviderLayer<P, T> for RethNodeLayer<EthApi>
where
    P: Provider<T>,
    T: Transport + Clone,
    EthApi: FullEthApiServer,
{
    type Provider = RethNodeProvider<P, T, EthApi>;

    fn layer(&self, inner: P) -> Self::Provider {
        RethNodeProvider::new(inner, self.eth_api().clone())
    }
}

#[derive(Clone, Debug)]
pub struct RethNodeProvider<P, T, EthApi: FullEthApiServer>
where
    EthApi: FullEthApiServer,
{
    inner: P,
    eth_api: EthApi,
    _t: PhantomData<T>,
}

impl<P, T, EthApi> RethNodeProvider<P, T, EthApi>
where
    EthApi: FullEthApiServer,
{
    /// Create a new `RethDbProvider` instance.
    pub fn new(inner: P, eth_api: EthApi) -> Self {
        Self { inner, eth_api, _t: PhantomData }
    }

    pub fn eth_api(&self) -> &EthApi {
        &self.eth_api
    }
}

impl<P, T, EthApi> Provider<T> for RethNodeProvider<P, T, EthApi>
where
    P: Provider<T>,
    T: Transport + Clone,
    EthApi: FullEthApiServer,
{
    fn root(&self) -> &RootProvider<T> {
        self.inner.root()
    }

    fn get_block_number(&self) -> ProviderCall<T, NoParams, U64, u64> {
        let block_number = self.eth_api.block_number().unwrap().to::<u64>();
        ProviderCall::ready(Ok(block_number))
    }
}
