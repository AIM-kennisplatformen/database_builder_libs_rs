use core::future::Future;

pub trait Connect {
    type Config;
    type Connected;
    type Error;

    fn connect(
        self,
        config: &Self::Config,
    ) -> impl Future<Output = Result<Self::Connected, Self::Error>> + Send;
}

pub trait Disconnect {
    type Output;

    fn disconnect(self) -> Self::Output;
}
