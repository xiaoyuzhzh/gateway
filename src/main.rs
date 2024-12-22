mod constant;

use std::collections::HashMap;
use std::env;
use async_trait::async_trait;
use pingora::prelude::*;
use std::sync::Arc;
use http::Uri;
use constant::*;

fn main() {
    env::set_var("RUST_LOG", "debug");
    env_logger::init();
    let mut my_server = Server::new(None).unwrap();
    my_server.bootstrap();

    // let mut upstreams =
    //     LoadBalancer::try_from_iter(["host.docker.internal:8081"]).unwrap();

    let proxy = MyProxy { lbs:get_lbs() };

    let mut lb = http_proxy_service(&my_server.configuration,proxy);
    lb.add_tcp("0.0.0.0:6188");

    my_server.add_service(lb);

    my_server.run_forever();
}


struct MyProxy {
    lbs: HashMap<String, Arc<LoadBalancer<RoundRobin>>>,
}

fn get_lbs() -> HashMap<String, Arc<LoadBalancer<RoundRobin>>> {
    // 设置不同路由的后端地址
    let mut lbs = HashMap::new();
    lbs.insert(
        LTC_2081.to_string(),
        Arc::new(
            LoadBalancer::<RoundRobin>::try_from_iter(vec![DEFAULT_HOST.to_string()+":9081"]).unwrap(),
        ),
    );
    lbs.insert(
        POWER_STATION.to_string(),
        Arc::new(
            LoadBalancer::<RoundRobin>::try_from_iter(vec![DEFAULT_HOST.to_string()+":7001"]).unwrap(),
        ),
    );
    lbs
}

#[async_trait]
impl ProxyHttp for MyProxy {
    type CTX = ();

    fn new_ctx(&self) -> Self::CTX {
        ()
    }

    async fn upstream_peer(
        &self,
        session: &mut Session,
        _ctx: &mut Self::CTX,
    ) -> pingora::Result<Box<HttpPeer>> {
        let original_uri = session.req_header().uri.clone();
        let original_path = original_uri.path();

        let (new_path, lb_key) = if original_path.starts_with(LTC_2081) {
            (&original_path[LTC_2081.len()..], LTC_2081)
        } else if original_path.starts_with(POWER_STATION) {
            (&original_path[POWER_STATION.len()..], POWER_STATION)
        } else {
            (&original_path[POWER_STATION.len()..], POWER_STATION)
        };

        let new_uri = Uri::builder()
            .scheme(original_uri.scheme().unwrap_or(&http::uri::Scheme::HTTP).as_str())
            .authority(
                original_uri
                    .authority()
                    .map(|a| a.as_str())
                    .unwrap_or(DEFAULT_HOST),
            )
            .path_and_query(new_path)
            .build()
            .unwrap();

        session.req_header_mut().set_uri(new_uri);

        let lb = self.lbs.get(lb_key).unwrap();

        let upstream = lb.select(b"", 256).unwrap();
        let peer = HttpPeer::new(upstream, false, String::new());
        Ok(Box::new(peer))
    }

    async fn request_filter(
        &self,
        session: &mut Session,
        _ctx: &mut Self::CTX,
    ) -> pingora::Result<bool> {
        Ok(false) // 允许继续处理
    }
}
