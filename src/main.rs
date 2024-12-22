mod constant;
mod proxy;

use pingora::prelude::*;
use std::env;

use crate::proxy::new;

fn main() {
    env::set_var("RUST_LOG", "debug");
    env_logger::init();
    let mut my_server = Server::new(None).unwrap();
    my_server.bootstrap();

    // let mut upstreams =
    //     LoadBalancer::try_from_iter(["host.docker.internal:8081"]).unwrap();

    let proxy = new();

    let mut lb = http_proxy_service(&my_server.configuration,proxy);
    lb.add_tcp("0.0.0.0:6188");

    my_server.add_service(lb);

    my_server.run_forever();
}



