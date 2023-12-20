mod onebot11;
mod onebot115;

use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;

use tokio::sync::RwLock;

use crate::{cqapi::cq_add_log_w, RT_PTR};

use self::{onebot11::OneBot11Connect, onebot115::OneBot115Connect};

#[async_trait]
trait BotConnectTrait:Send + Sync {
    fn get_platform(&self) -> Vec<String>;
    async fn call_api(&mut self,platform:&str,self_id:&str,json:&mut serde_json::Value) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>>;
    fn get_self_id(&self) -> Vec<String>;
    fn get_url(&self) -> String;
    fn get_alive(&self) -> bool;
    async fn connect(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    async fn disconnect(&mut self);
}


lazy_static! {
    static ref G_BOT_MAP:RwLock<HashMap<String,Arc<RwLock<dyn BotConnectTrait>>>> = RwLock::new(HashMap::new());
}


pub async fn call_api(platform:&str,self_id:&str,json:&mut serde_json::Value) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
    for bot in &*G_BOT_MAP.read().await {
        if bot.1.read().await.get_platform().contains(&platform.to_owned()) && bot.1.read().await.get_self_id().contains(&self_id.to_owned()) {
            let bot_select = bot.1.clone();
            let mut bot2 = bot_select.write().await;
            return bot2.call_api(platform,self_id, json).await;
        }
    }
    cq_add_log_w(&format!("no such bot:{platform},{self_id}")).unwrap();
    return Ok(serde_json::json!(""));
}


pub fn do_conn_event() -> Result<i32, Box<dyn std::error::Error>> {
    std::thread::spawn(move ||{
        loop {
            // 得到配置文件中的url
            let config = crate::read_config().unwrap();
            let urls_val = config.get("ws_urls").ok_or("无法获取ws_urls").unwrap().as_array().ok_or("无法获取web_host").unwrap().to_owned();
            let mut config_urls = vec![];
            for url in &urls_val {
                let url_str = url.as_str().ok_or("ws_url不是字符数组").unwrap().to_string();
                config_urls.push(url_str);
            }
            
            RT_PTR.clone().block_on(async move {
                // 删除所有不在列表中的url和死去的bot
                {
                    let mut earse_vec = vec![];
                    let mut bot_map = G_BOT_MAP.write().await;
                    for (url,bot) in &*bot_map {
                        if !config_urls.contains(url) || bot.read().await.get_alive() == false {
                            bot.write().await.disconnect().await;
                            earse_vec.push(url.clone());
                        }
                    }
                    for url in &earse_vec {
                        bot_map.remove(url);
                    }
                }
                // 连接未在bot_map中的url
                for url in &config_urls {
                    let is_exist;
                    if G_BOT_MAP.read().await.contains_key(url) {
                        is_exist = true;
                    }else{
                        is_exist = false;
                    }
                    if !is_exist {
                        let url_t = url.clone();
                        RT_PTR.clone().spawn(async move {
                            if url_t.starts_with("ws://") || url_t.starts_with("wss://") {
                                let mut bot = OneBot11Connect::build(&url_t);
                                if let Err(_err) = bot.connect().await {
                                    cq_add_log_w(&format!("连接到onebot失败:{}",url_t)).unwrap();
                                } else {
                                    G_BOT_MAP.write().await.insert(url_t,Arc::new(RwLock::new(bot)));
                                }
                            }else if url_t.starts_with("ovo://")  || true {
                                let mut bot = OneBot115Connect::build(&url_t);
                                if let Err(err) = bot.connect().await {
                                    cq_add_log_w(&format!("连接到ovo失败:{url_t},{err:?}")).unwrap();
                                } else {
                                    G_BOT_MAP.write().await.insert(url_t,Arc::new(RwLock::new(bot)));
                                }
                            }
                            
                        });
                    }
                }
            });
            
            std::thread::sleep(std::time::Duration::from_secs(5));
        }
    });
    Ok(0)
}