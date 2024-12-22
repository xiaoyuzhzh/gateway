use std::collections::HashMap;
use std::env;
use std::sync::Arc;
use async_trait::async_trait;
use config::{Config, ConfigError, File, FileFormat};
use http::Uri;
use pingora::ErrorType;
use pingora::lb::LoadBalancer;
use pingora::prelude::{HttpPeer, ProxyHttp, RoundRobin, Session};
use serde::Deserialize;
use crate::constant::{DEFAULT_HOST, LTC_2081, POWER_STATION};

pub struct MyProxy {
    lbs: HashMap<String, Arc<LoadBalancer<RoundRobin>>>,
    settings: Settings,
}

pub fn new() -> MyProxy {
    MyProxy {
        lbs: get_lbs(),
        settings: load_configs().unwrap(),
    }
}

fn get_lbs() -> HashMap<String, Arc<LoadBalancer<RoundRobin>>> {

    let settings = load_configs().unwrap_or_else(|e| {
        eprintln!("Failed to load config: {:?}", e);
        panic!("load config failed");
    });

    // 设置不同路由的后端地址
    let mut lbs = HashMap::new();
    for (route, config) in settings.routes {
        lbs.insert(
            route,
            Arc::new(
                LoadBalancer::<RoundRobin>::try_from_iter(vec![config.forward_to]).unwrap(),
            ),
        );
    }
    lbs
}

#[derive(Debug, Deserialize)]
struct RouteConfig {
    forward_to: String,
    replace_path: String,
}

#[derive(Debug, Deserialize)]
struct Settings {
    routes: HashMap<String,RouteConfig>,
}


fn load_configs()-> Result<Settings,ConfigError>{
    let mut settings = Config::default();

    let config_path = if let Ok(config_path) = env::var("CONFIG_PATH"){
        config_path
    }else {
        "/app/config/config.yaml".to_string()
    };

    settings.merge(File::new(&*config_path, FileFormat::Yaml))?;
    let setting = settings.try_deserialize::<Settings>()?;

    println!("config: {:?}", setting);
    Ok(setting)
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

        println!("routes: {:?}", self.settings.routes);

        // 检查是否有任何路由键是当前请求路径的前缀
        if let Some((route, route_config)) = self.settings.routes.iter().find(|(key, _)| original_path.starts_with(&**key)) {
            println!("Value of str: '{}'", route);
            let new_path = original_path.replacen(route, &route_config.replace_path, 1); // 替换匹配的部分路径
            let new_uri = Uri::builder()
                .scheme(original_uri.scheme().unwrap_or(&http::uri::Scheme::HTTP).as_str())
                .authority(route_config.forward_to.clone())
                .path_and_query(new_path)
                .build()
                .unwrap();

            session.req_header_mut().set_uri(new_uri);

            let lb = self.lbs.get(route).unwrap();
            let upstream = lb.select(b"", 256).unwrap();
            let peer = HttpPeer::new(upstream, false, String::new());
            return Ok(Box::new(peer));
        }

        Err(pingora::Error::new(ErrorType::new("not found")))
    }

    async fn request_filter(
        &self,
        session: &mut Session,
        _ctx: &mut Self::CTX,
    ) -> pingora::Result<bool> {
        Ok(false) // 允许继续处理
    }
}