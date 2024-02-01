use std::collections::HashMap;
use std::io::Read;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::thread;
use crate::cqapi::{cq_get_app_directory1, get_history_log, cq_add_log};
use crate::cqevent::do_script;
use crate::httpevent::do_http_event;
use crate::mytool::read_json_str;
use crate::{read_config, G_AUTO_CLOSE, CLEAR_UUID};
use crate::redlang::RedLang;
use crate::{cqapi::cq_add_log_w, RT_PTR};
use futures_util::{SinkExt, StreamExt};
use http_body_util::BodyExt;
use hyper::body::Incoming;
use hyper::http::HeaderValue;
use hyper::service::service_fn;
use hyper::Response;
use serde_json::json;
use tokio_util::bytes::Buf;
use bytes::Bytes;
use futures_util::TryStreamExt;


type GenericError = Box<dyn std::error::Error + Send + Sync>;
type BoxBody = http_body_util::combinators::BoxBody<Bytes,GenericError>;
type Result<T> = std::result::Result<T, GenericError>;

lazy_static! {
    static ref G_LOG_MAP:tokio::sync::RwLock<HashMap<String,tokio::sync::mpsc::Sender<String>>> = tokio::sync::RwLock::new(HashMap::new());
    pub static ref G_PY_ECHO_MAP:tokio::sync::RwLock<HashMap<String,tokio::sync::mpsc::Sender<String>>> = tokio::sync::RwLock::new(HashMap::new());
    pub static ref G_PY_HANDER:tokio::sync::RwLock<Option<tokio::sync::mpsc::Sender<String>>> = tokio::sync::RwLock::new(None);
    pub static ref G_PYSER_OPEN:AtomicBool = AtomicBool::new(false);
}

pub fn add_ws_log(log_msg:String) {
    RT_PTR.clone().spawn(async move {
        let lk = G_LOG_MAP.read().await;
        for (_,tx) in &*lk {
            let _foo = tx.send(log_msg.clone()).await;
        }
    });
}

async fn deal_api(request: hyper::Request<hyper::body::Incoming>,can_write:bool,can_read:bool) -> Result<hyper::Response<BoxBody>> {
    let url_path = request.uri().path();
    if url_path == "/get_code" {
        if !can_read {
            let res = hyper::Response::new(full("api not found"));
            return Ok(res);
        }
        match crate::read_code_cache() {
            Ok(code) => {
                let ret = json!({
                    "retcode":0,
                    "data":code
                });
                let mut res = hyper::Response::new(full(ret.to_string()));
                res.headers_mut().insert("Content-Type", HeaderValue::from_static("application/json"));
                Ok(res)
            },
            Err(err) => {
                let mut res = hyper::Response::new(full(err.to_string()));
                *res.status_mut() = hyper::StatusCode::INTERNAL_SERVER_ERROR;
                Ok(res)
            },
        }
    }
    else if url_path == "/read_one_pkg" {
        if !can_read {
            let res = hyper::Response::new(full("api not found"));
            return Ok(res);
        }
        let params = crate::httpevent::get_params_from_uri(request.uri());
        let pkg_name;
        if let Some(name) = params.get("pkg_name") {
            pkg_name = name.to_owned();
        }else {
            pkg_name = "".to_owned();
        }
        match crate::read_one_pkg(&pkg_name) {
            Ok(code) => {
                let ret = json!({
                    "retcode":0,
                    "data":code
                });
                let mut res = hyper::Response::new(full(ret.to_string()));
                res.headers_mut().insert("Content-Type", HeaderValue::from_static("application/json"));
                Ok(res)
            },
            Err(err) => {
                let mut res = hyper::Response::new(full(err.to_string()));
                *res.status_mut() = hyper::StatusCode::INTERNAL_SERVER_ERROR;
                Ok(res)
            },
        }
    }
    else if url_path == "/get_all_pkg_name" {
        if !can_read {
            let res = hyper::Response::new(full("api not found"));
            return Ok(res);
        }
        match crate::get_all_pkg_name_by_cache() {
            Ok(code) => {
                let ret = json!({
                    "retcode":0,
                    "data":code
                });
                let mut res = hyper::Response::new(full(ret.to_string()));
                res.headers_mut().insert("Content-Type", HeaderValue::from_static("application/json"));
                Ok(res)
            },
            Err(err) => {
                let mut res = hyper::Response::new(full(err.to_string()));
                *res.status_mut() = hyper::StatusCode::INTERNAL_SERVER_ERROR;
                Ok(res)
            },
        }
    }else if url_path == "/get_config" {
        if !can_read {
            let res = hyper::Response::new(full("api not found"));
            return Ok(res);
        }
        match crate::read_config() {
            Ok(code) => {
                let ret = json!({
                    "retcode":0,
                    "data":code
                });
                let mut res = hyper::Response::new(full(ret.to_string()));
                res.headers_mut().insert("Content-Type", HeaderValue::from_static("application/json"));
                Ok(res)
            },
            Err(err) => {
                let mut res = hyper::Response::new(full(err.to_string()));
                *res.status_mut() = hyper::StatusCode::INTERNAL_SERVER_ERROR;
                Ok(res)
            },
        }
    }else if url_path == "/set_ws_urls" {
        if can_write == false {
            let res = hyper::Response::new(full("api not found"));
            return Ok(res);
        }
        let body = request.collect().await?.aggregate().reader();
        let js:serde_json::Value = serde_json::from_reader(body)?;
        match crate::set_ws_urls(js){
            Ok(_) => {
                let ret = json!({
                    "retcode":0,
                });
                let mut res = hyper::Response::new(full(ret.to_string()));
                res.headers_mut().insert("Content-Type", HeaderValue::from_static("application/json"));
                Ok(res)
            },
            Err(_) => {
                let ret = json!({
                    "retcode":-1,
                });
                let mut res = hyper::Response::new(full(ret.to_string()));
                res.headers_mut().insert("Content-Type", HeaderValue::from_static("application/json"));
                Ok(res)
            },
        }
        
    }else if url_path == "/set_code" {
        if can_write == false {
            let res = hyper::Response::new(full("api not found"));
            return Ok(res);
        }
        let body = request.collect().await?.aggregate().reader();
        let js:serde_json::Value = serde_json::from_reader(body)?;
        let (tx, rx) =  tokio::sync::oneshot::channel();
        tokio::task::spawn_blocking(move || {
            let rst = crate::save_code(&js.to_string());
            if rst.is_ok() {
                let ret = json!({
                    "retcode":0,
                });
                let rst = tx.send(ret);
                if rst.is_err() {
                    cq_add_log_w(&format!("Error:{:?}",rst.err())).unwrap();
                }
            }
            else {
                let ret = json!({
                    "retcode":-1,
                });
                let rst = tx.send(ret);
                if rst.is_err() {
                    cq_add_log_w(&format!("Error:{:?}",rst.err())).unwrap();
                }
            }
        }).await?;
        let ret = rx.await?;
        let mut res = hyper::Response::new(full(ret.to_string()));
        res.headers_mut().insert("Content-Type", HeaderValue::from_static("application/json"));
        Ok(res)    
    }
    else if url_path == "/save_one_pkg" {
        if can_write == false {
            let res = hyper::Response::new(full("api not found"));
            return Ok(res);
        }
        let body = request.collect().await?.aggregate().reader();
        let js:serde_json::Value = serde_json::from_reader(body)?;
        let (tx, rx) =  tokio::sync::oneshot::channel();
        tokio::task::spawn_blocking(move || {
            let rst = crate::save_one_pkg(&js.to_string());
            if rst.is_ok() {
                let ret = json!({
                    "retcode":0,
                });
                let rst = tx.send(ret);
                if rst.is_err() {
                    cq_add_log_w(&format!("Error:{:?}",rst.err())).unwrap();
                }
            }
            else {
                let ret = json!({
                    "retcode":-1,
                });
                let rst = tx.send(ret);
                if rst.is_err() {
                    cq_add_log_w(&format!("Error:{:?}",rst.err())).unwrap();
                }
            }
        }).await?;
        let ret = rx.await?;
        let mut res = hyper::Response::new(full(ret.to_string()));
        res.headers_mut().insert("Content-Type", HeaderValue::from_static("application/json"));
        Ok(res)    
    }
    else if url_path == "/rename_one_pkg" {
        if can_write == false {
            let res = hyper::Response::new(full("api not found"));
            return Ok(res);
        }
        let body = request.collect().await?.aggregate().reader();
        let js:serde_json::Value = serde_json::from_reader(body)?;
        let old_pkg_name = read_json_str(&js, "old_pkg_name");
        let new_pkg_name = read_json_str(&js, "new_pkg_name");
        let (tx, rx) =  tokio::sync::oneshot::channel();
        tokio::task::spawn_blocking(move || {
            let rst = crate::rename_one_pkg(&old_pkg_name,&new_pkg_name);
            if rst.is_ok() {
                let ret = json!({
                    "retcode":0,
                });
                let rst = tx.send(ret);
                if rst.is_err() {
                    cq_add_log_w(&format!("Error:{:?}",rst.err())).unwrap();
                }
            }
            else {
                let ret = json!({
                    "retcode":-1,
                });
                let rst = tx.send(ret);
                if rst.is_err() {
                    cq_add_log_w(&format!("Error:{:?}",rst.err())).unwrap();
                }
            }
        }).await?;
        let ret = rx.await?;
        let mut res = hyper::Response::new(full(ret.to_string()));
        res.headers_mut().insert("Content-Type", HeaderValue::from_static("application/json"));
        Ok(res)    
    }
    else if url_path == "/del_one_pkg" {
        if can_write == false {
            let res = hyper::Response::new(full("api not found"));
            return Ok(res);
        }
        let body = request.collect().await?.aggregate().reader();
        let js:serde_json::Value = serde_json::from_reader(body)?;
        let pkg_name = read_json_str(&js, "pkg_name");
        let (tx, rx) =  tokio::sync::oneshot::channel();
        tokio::task::spawn_blocking(move || {
            let rst = crate::del_one_pkg(&pkg_name);
            if rst.is_ok() {
                let ret = json!({
                    "retcode":0,
                });
                let rst = tx.send(ret);
                if rst.is_err() {
                    cq_add_log_w(&format!("Error:{:?}",rst.err())).unwrap();
                }
            }
            else {
                let ret = json!({
                    "retcode":-1,
                });
                let rst = tx.send(ret);
                if rst.is_err() {
                    cq_add_log_w(&format!("Error:{:?}",rst.err())).unwrap();
                }
            }
        }).await?;
        let ret = rx.await?;
        let mut res = hyper::Response::new(full(ret.to_string()));
        res.headers_mut().insert("Content-Type", HeaderValue::from_static("application/json"));
        Ok(res)    
    }
    else if url_path == "/run_code" {
        if can_write == false {
            let res = hyper::Response::new(full("api not found"));
            return Ok(res);
        }
        let body = request.collect().await?.aggregate().reader();
        let root:serde_json::Value = serde_json::from_reader(body)?;

        let bot_id = read_json_str(&root, "bot_id");
        let platform = read_json_str(&root, "platform");
        let user_id = read_json_str(&root, "user_id");
        let group_id = read_json_str(&root, "group_id");
        let code = read_json_str(&root, "content");
        thread::spawn(move ||{
            let mut rl = RedLang::new();
            rl.set_exmap("机器人ID", &bot_id).unwrap();
            rl.set_exmap("群ID", &group_id).unwrap();
            rl.set_exmap("发送者ID", &user_id).unwrap();
            rl.set_exmap("机器人平台", &platform).unwrap();
            rl.pkg_name = "".to_owned(); // 默认包
            rl.script_name = "网页调试".to_owned();
            rl.can_wrong = true;
            if let Err(err) = do_script(&mut rl, &code) {
                cq_add_log_w(&format!("{}",err)).unwrap();
            }
        });
        let mut res = hyper::Response::new(full(serde_json::json!({
            "retcode":0,
        }).to_string()));
        res.headers_mut().insert("Content-Type", HeaderValue::from_static("application/json"));
        Ok(res)    
    }
    else if url_path == "/run_code_and_ret" {
        if can_write == false {
            let res = hyper::Response::new(full("api not found"));
            return Ok(res);
        }
        let body = request.collect().await?.aggregate().reader();
        let root:serde_json::Value = serde_json::from_reader(body)?;

        let bot_id = read_json_str(&root, "bot_id");
        let platform = read_json_str(&root, "platform");
        let user_id = read_json_str(&root, "user_id");
        let group_id = read_json_str(&root, "group_id");
        let code = read_json_str(&root, "content");

        
        let (tx, rx) =  tokio::sync::oneshot::channel();
        tokio::task::spawn_blocking(move ||{
            let mut rl = RedLang::new();
            rl.set_exmap("机器人ID", &bot_id).unwrap();
            rl.set_exmap("群ID", &group_id).unwrap();
            rl.set_exmap("发送者ID", &user_id).unwrap();
            rl.set_exmap("机器人平台", &platform).unwrap();
            rl.pkg_name = "".to_owned(); // 默认包
            rl.script_name = "网页调试".to_owned();
            rl.can_wrong = true;
            let ret_rst = rl.parse(&code);
            let res;
            let ret;
            if let Ok(ret_str) = ret_rst {
                // 处理清空指令
                if let Some(pos) = ret_str.rfind(CLEAR_UUID.as_str()) {
                    ret = ret_str.get((pos + 36)..).unwrap().to_owned();
                } else {
                    ret = ret_str;
                }
                res = serde_json::json!({
                    "retcode":0,
                    "data":ret
                }).to_string();
            } else {
                ret = format!("{}",ret_rst.err().unwrap().to_string());
                res = serde_json::json!({
                    "retcode":-1,
                    "data":ret
                }).to_string();
            }
            let rst = tx.send(res);
            if rst.is_err() {
                cq_add_log_w(&format!("Error:{:?}",rst.err())).unwrap();
            }
        });
        let ret_str = rx.await?;
        let mut res = hyper::Response::new(full(ret_str));
        res.headers_mut().insert("Content-Type", HeaderValue::from_static("application/json"));
        Ok(res)    
    }
    else if url_path == "/close" {
        if can_write == false {
            let res = hyper::Response::new(full("api not found"));
            return Ok(res);
        }
        cq_add_log_w("收到退出指令，正在退出").unwrap();
        crate::wait_for_quit();
    }else if url_path == "/get_version" {
        if !can_read {
            let res = hyper::Response::new(full("api not found"));
            return Ok(res);
        }
        let ret = json!({
            "retcode":0,
            "data":crate::get_version()
        });
        let mut res = hyper::Response::new(full(ret.to_string()));
        res.headers_mut().insert("Content-Type", HeaderValue::from_static("application/json"));
        Ok(res)
    }else if url_path == "/login" {
        let body_len = request.headers().get("content-length").ok_or("/login 中没有content-length")?.to_str()?.parse::<usize>()?;
        let mut res = hyper::Response::new(full(vec![]));
        if body_len < 256 {
            let mut body_reader = request.collect().await?.aggregate().reader();
            let mut body = Vec::new();
            body_reader.read_to_end(&mut body)?;
            //let body = hyper::body::to_bytes(request.into_body()).await?;
            *res.status_mut() = hyper::StatusCode::MOVED_PERMANENTLY;
            let pass_cookie = format!("{};Max-Age=31536000",String::from_utf8(body.to_vec())?);
            res.headers_mut().append(hyper::header::SET_COOKIE, HeaderValue::from_str(&pass_cookie)?);
        }
        res.headers_mut().insert("Location", HeaderValue::from_static("/index.html"));
        Ok(res)
    }
    else if url_path == "/event" {
        if can_write == false {
            let res = hyper::Response::new(full("api not found"));
            return Ok(res);
        }
        let mp = crate::httpevent::get_params_from_uri(request.uri());
        let flag;
        if let Some(f) = mp.get("flag") {
            flag = f.as_str();
        }else{
            flag = "";
        }
        let ret_json = crate::openapi::get_event(flag);
        let mut res = hyper::Response::new(full(ret_json.to_string()));
        res.headers_mut().append(hyper::header::CONTENT_TYPE,HeaderValue::from_str("application/json")?);
        Ok(res)
    }
    else if url_path == "/api" {
        if can_write == false {
            let res = hyper::Response::new(full("api not found"));
            return Ok(res);
        }
        let whole_body = request.collect().await?.aggregate().reader();
        let root:serde_json::Value = serde_json::from_reader(whole_body)?;
        let platform = read_json_str(&root, "platform");
        let self_id = read_json_str(&root, "self_id");
        let passive_id = read_json_str(&root, "passive_id");
        let (tx, rx) =  tokio::sync::oneshot::channel();
        tokio::task::spawn_blocking(move ||{
            let rst = crate::cqapi::cq_call_api(&platform,&self_id,&passive_id,&root.to_string());
            if let Err(e) = rst {
                crate::cqapi::cq_add_log(format!("{:?}", e).as_str()).unwrap();
                let _ = tx.send(serde_json::json!({
                    "retcode":-1,
                    "data":format!("{e:?}")
                }).to_string());
            }else {
                let ret = rst.unwrap();
                let _ = tx.send(ret);
            }
        });
        let ret = rx.await?;
        let mut res = hyper::Response::new(full(ret));
        res.headers_mut().append(hyper::header::CONTENT_TYPE,HeaderValue::from_str("application/json")?);
        Ok(res)
    }
    else if url_path.starts_with("/user") {
        let (tx, rx) =  tokio::sync::oneshot::channel();
        tokio::task::spawn_blocking(move || {
            let ret = do_http_event(request,can_write,can_read);
            if ret.is_ok() {
                let rst = tx.send(ret.unwrap());
                if rst.is_err() {
                    cq_add_log_w(&format!("Error:{:?}",rst.err())).unwrap();
                }
            }else {
                let err_str = format!("Error:{:?}",ret);
                let mut res = hyper::Response::new(full(err_str));
                res.headers_mut().insert("Content-Type", HeaderValue::from_static("text/html; charset=utf-8"));
                let rst = tx.send(res);
                if rst.is_err() {
                    cq_add_log_w(&format!("Error:{:?}",rst.err())).unwrap();
                }
            }
        }).await?;
        let ret = rx.await?;
        // let t = http_body_util::combinators::BoxBody::new(ret);
        // let k = ret.map_err(|never| match never {});
        Ok(ret)
    }
    else{
        let res = hyper::Response::new(full("api not found"));
        Ok(res)
    }
}

async fn deal_file(request: hyper::Request<hyper::body::Incoming>) -> Result<hyper::Response<BoxBody>> {
    let url_path = request.uri().path();
    let app_dir = cq_get_app_directory1().unwrap();
    let path = PathBuf::from(&app_dir);
    let path = path.join("webui");
    let url_path_t = url_path.replace("/", &std::path::MAIN_SEPARATOR.to_string());
    let file_path = path.join(url_path_t.get(1..).unwrap());
    let file = tokio::fs::File::open(file_path).await?;
    let reader_stream = tokio_util::io::ReaderStream::new(file);
    let stream_body = http_body_util::StreamBody::new(reader_stream.map_ok(hyper::body::Frame::data));
    let boxed_body: http_body_util::combinators::BoxBody<tokio_util::bytes::Bytes, std::io::Error> = BodyExt::boxed(stream_body);
    let kk = boxed_body.map_err(|e|Box::new(e) as Box<dyn std::error::Error + Send + Sync>).boxed();
    let mut res = hyper::Response::new(kk);
    *res.status_mut() = hyper::StatusCode::OK;
    if url_path.ends_with(".html") {
        res.headers_mut().insert("Cache-Control", HeaderValue::from_static("max-age=300"));
        res.headers_mut().insert("Content-Type", HeaderValue::from_static("text/html; charset=utf-8"));
    }else if url_path.ends_with(".js") {
        res.headers_mut().insert("Cache-Control", HeaderValue::from_static("max-age=300"));
        res.headers_mut().insert("Content-Type", HeaderValue::from_static("text/javascript; charset=utf-8"));
    }else if url_path.ends_with(".css") {
        res.headers_mut().insert("Cache-Control", HeaderValue::from_static("max-age=300"));
        res.headers_mut().insert("Content-Type", HeaderValue::from_static("text/css; charset=utf-8"));
    }else if url_path.ends_with(".png") {
        res.headers_mut().insert("Cache-Control", HeaderValue::from_static("max-age=300"));
        res.headers_mut().insert("Content-Type", HeaderValue::from_static("image/png"));
    }else if url_path.ends_with(".txt") {
        res.headers_mut().insert("Content-Type", HeaderValue::from_static("text/plain"));
    }else if url_path.ends_with(".md") {
        res.headers_mut().insert("Content-Type", HeaderValue::from_static("text/markdown"));
    }
    else {
        *res.status_mut() = hyper::StatusCode::NOT_FOUND;
    }
    // cq_add_log_w(&format!("{:?}",res));
    Ok(res)
}
/// 处理ws协议
async fn serve_websocket(websocket: hyper_tungstenite::HyperWebsocket,mut rx:tokio::sync::mpsc::Receiver<String>) -> Result<()> {
    
    // 获得升级后的ws流
    let ws_stream = websocket.await?;
    let (mut write_half, mut _read_half ) = futures_util::StreamExt::split(ws_stream);
    
    while let Some(msg) = rx.recv().await { // 当所有tx被释放时,tx.recv会返回None
        // log::info!("recive:{}",msg);
        write_half.send(hyper_tungstenite::tungstenite::Message::Text(msg.to_string())).await?;
    }
    Ok(())
}


/// 处理ws协议
async fn serve_py_websocket(websocket: hyper_tungstenite::HyperWebsocket,mut rx:tokio::sync::mpsc::Receiver<String>) -> Result<()> {
    
    cq_add_log("connect to pyserver").unwrap();

    // 获得升级后的ws流
    let ws_stream = websocket.await?;
    let (mut write_half, read_half ) = futures_util::StreamExt::split(ws_stream);
    
    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await { // 当所有tx被释放时,tx.recv会返回None
            // log::info!("recive:{}",msg);
            let rst = write_half.send(hyper_tungstenite::tungstenite::Message::Text(msg.to_string())).await;
            if rst.is_err() {
                let mut lk = G_PY_HANDER.write().await;
                (*lk) = None;
                cq_add_log_w("serve send1 of python err").unwrap();
                break;
            }
        }
        cq_add_log_w("serve send2 of python err").unwrap();
    });

    async fn deal_msg(mut read_half:futures_util::stream::SplitStream<tokio_tungstenite::WebSocketStream<hyper_util::rt::TokioIo<hyper::upgrade::Upgraded>>>) -> Result<()> {
        while let Some(msg_t) = read_half.next().await {
            {
                let lk = G_PY_HANDER.read().await;
                if lk.is_none() {
                    break;
                }
            }
            let msg_text = msg_t?.to_text()?.to_owned();

            if msg_text == "opened" {
                G_PYSER_OPEN.store(true,std::sync::atomic::Ordering::Relaxed);
                cq_add_log_w("python环境已经连接！").unwrap();
                continue;
            }

            let js:serde_json::Value = serde_json::from_str(&msg_text)?;
            let echo = read_json_str(&js, "echo");
            let lk = G_PY_ECHO_MAP.read().await;
            if lk.contains_key(&echo) {
                lk.get(&echo).unwrap().send(read_json_str(&js,"data")).await?;
            }
        }
        Ok(())
    }
    let ret = deal_msg(read_half).await;
    let mut lk = G_PY_HANDER.write().await;
    if ret.is_err() {
        cq_add_log_w("serve recv of python err").unwrap();
    }
    (*lk) = None;
    
    Ok(())
}


fn http_auth(request: &hyper::Request<Incoming>) -> Result<i32> {
    let web_pass_raw = crate::read_web_password()?;
    if web_pass_raw == "" {
        return Ok(2)
    }
    let headers = request.headers();
    let cookie_str = headers.get("Cookie").ok_or("can not found Cookie")?.to_str()?;
    {
        let web_pass:String = url::form_urlencoded::byte_serialize(web_pass_raw.as_bytes()).collect();
        if cookie_str.contains(&format!("password={}",web_pass)) {
            return Ok(2)
        }
    }
    let read_only_web_pass_raw = crate::read_readonly_web_password()?;
    if read_only_web_pass_raw == "" {
        return Ok(1)
    }
    {
        let web_pass:String = url::form_urlencoded::byte_serialize(read_only_web_pass_raw.as_bytes()).collect();
        if cookie_str.contains(&format!("password={}",web_pass)) {
            return Ok(1)
        }
    }
    return Err("password invaild".into());
}


fn is_users_api(url_path:&str) -> bool {
    if url_path.starts_with("/user") {
        return true;
    }
    return false;
}

fn rout_to_login() -> hyper::Response<BoxBody> {
    let mut res = hyper::Response::new(full(vec![]));
    *res.status_mut() = hyper::StatusCode::MOVED_PERMANENTLY;
    res.headers_mut().insert("Location", HeaderValue::from_static("/login.html"));
    return res;
}

pub fn full<T: Into<Bytes>>(chunk: T) -> BoxBody {
    http_body_util::Full::new(chunk.into())
        .map_err(|never| match never {})
        .boxed()
}
async fn connect_handle(request: hyper::Request<hyper::body::Incoming>,is_local: bool) -> Result<Response<BoxBody>> {
    
    let url_path = request.uri().path();

    // 登录页面不进行身份验证
    if url_path == "/login.html" {
        return deal_file(request).await; 
    }

    // 登录API不进行身份验证
    if url_path == "/login" {
        return deal_api(request,false,false).await;
    }

    // 心跳接口无须身份验证
    if url_path == "/heartbeat" {
        let mut lk = G_AUTO_CLOSE.lock().unwrap();
        (*lk) = false;
        
        let res = hyper::Response::new(full("ok"));
        return Ok(res);
    }

    
    // 获取身份信息
    let can_write;
    let can_read;
    if !is_local {
    // if !addr.ip().is_loopback() {
        let http_auth_rst = http_auth(&request);
        if http_auth_rst.is_err() {
            can_read = false;
            can_write = false;
        }else {
            can_read = true;
            let auth_code = http_auth_rst.unwrap();
            if auth_code == 2 {
                can_write = true;
            }else {
                can_write = false;
            }
        }
    }else{ // 本机地址不进行身份验证
        can_write = true;
        can_read = true;
    }

    // 认证失败,且不是用户API,跳转登录页面
    if !can_read {
        if !is_users_api(url_path) {
            return Ok(rout_to_login());
        }
    }


    // 升级ws协议
    if hyper_tungstenite::is_upgrade_request(&request) {
        if url_path == "/watch_log" {

            // 没有写权限不允许访问log
            if can_write == false {
                let res = hyper::Response::new(full("api not found"));
                return Ok(res);
            }

            // ws协议升级返回
            let (response, websocket) = hyper_tungstenite::upgrade(request, None)?;
            let (tx, rx) =  tokio::sync::mpsc::channel::<String>(60);
            
            let history_log = get_history_log();
            for it in history_log {
                tx.send(it).await?;
            }

            // 开启一个线程来处理ws
            tokio::spawn(async move {
                let uid = uuid::Uuid::new_v4().to_string();
                {
                    let mut lk = G_LOG_MAP.write().await;
                    lk.insert(uid.clone(), tx);
                }
                if let Err(e) = serve_websocket(websocket,rx).await {
                    log::warn!("Error in websocket connection: {}", e);
                }
                // 线程结束，删除对应的entry
                let mut lk = G_LOG_MAP.write().await;
                lk.remove(&uid);
            });
            let headers = response.headers();
            let mut rrr = 
                Response::builder()
                .body(full("switching to websocket protocol"))?;
            (*rrr.status_mut()) = response.status();
            (*rrr.headers_mut()) = headers.to_owned();
            return Ok(rrr);
        } else if url_path == "/pyserver" {
            // 没有写权限不允许访问pyserver
            if can_write == false {
                let res = hyper::Response::new(full("api not found"));
                return Ok(res);
            }
            // ws协议升级返回
            let (response, websocket) = hyper_tungstenite::upgrade(request, None)?;

            // 开启一个线程来处理ws
            tokio::spawn(async move {
                let (tx, rx) =  tokio::sync::mpsc::channel::<String>(60);
                {
                    let mut k = G_PY_HANDER.write().await;
                    *k = Some(tx);
                }
                if let Err(e) = serve_py_websocket(websocket,rx).await {
                    cq_add_log_w(&format!("Error in websocket connection: {}", e)).unwrap();
                }
            });
            
            let headers = response.headers();
            let mut rrr = 
                Response::builder()
                .body(full("switching to websocket protocol"))?;
            (*rrr.status_mut()) = response.status();
            (*rrr.headers_mut()) = headers.to_owned();
            return Ok(rrr);
        } else {
            return Ok(hyper::Response::new(full("broken http")));
        }
    }else {
        // 处理HTTP协议
        if url_path == "/" {

            // 没有读权限不允许访问主页
            if !can_read {
                return Ok(rout_to_login());
            }
            let mut res = hyper::Response::new(full(vec![]));
            *res.status_mut() = hyper::StatusCode::MOVED_PERMANENTLY;
            res.headers_mut().insert("Location", HeaderValue::from_static("/index.html"));
            return Ok(res);
        } else if url_path == "/favicon.ico" {
            // 没有读权限不允许访问图标
            if !can_read {
                return Ok(rout_to_login());
            }
            let app_dir = cq_get_app_directory1().unwrap();
            let path = PathBuf::from(&app_dir);
            let path = path.join("webui");
            let url_path_t = "favicon.ico".to_owned();
            let file_path = path.join(url_path_t);
            let file_buf = tokio::fs::read(&file_path).await?;
            let mut res = hyper::Response::new(full(file_buf));
            res.headers_mut().insert("Content-Type", HeaderValue::from_static("image/x-icon"));
            return Ok(res);
        } else if !url_path.contains(".") || url_path.starts_with("/user") {
            return deal_api(request,can_write,can_read).await;
        } else {
            // 没有读权限不允许访问文件
            if !can_read {
                return Ok(rout_to_login());
            }
            return deal_file(request).await;
        }
    }

    
}

pub fn init_http_server() -> Result<()> {
    let config = read_config()?;
    let mut host = config.get("web_host").ok_or("无法获取web_host")?.as_str().ok_or("无法获取web_host")?;
    let port = config.get("web_port").ok_or("无法获取web_port")?.as_u64().ok_or("无法获取web_port")?;
    if host == "localhost" {
        host = "127.0.0.1";
    }
    let web_uri = format!("{host}:{port}");
    cq_add_log_w(&format!("webui访问地址：http://{web_uri}")).unwrap();
    let addr1 = web_uri.parse::<std::net::SocketAddr>().unwrap();

    RT_PTR.spawn(async move {
        let socket = tokio::net::TcpSocket::new_v4().unwrap();
        
        // win 下不需要设置这个
        #[cfg(not(windows))]
        socket.set_reuseaddr(true).unwrap();

        socket.bind(addr1).unwrap();
        let listener = socket.listen(20).unwrap();
        

        loop {
            let (stream, remote_address) = listener.accept().await.unwrap();

            let io = hyper_util::rt::TokioIo::new(stream);
            
            let service = service_fn(move |req| {
                connect_handle(req,remote_address.ip().is_loopback())
            });
            
            tokio::task::spawn(async move {
                if let Err(err) = hyper::server::conn::http1::Builder::new()
                    .serve_connection(io, service)
                    .with_upgrades()
                    .await
                {
                    cq_add_log_w(&format!("Error serving connection: {:?}", err)).unwrap();
                }
            });
        } 
    });
    
    if let Some(not_open_browser) = config.get("not_open_browser") {
        if not_open_browser == false {
            opener::open(format!("http://localhost:{port}"))?;
        }
    }else {
        opener::open(format!("http://localhost:{port}"))?;
    }
    
    Ok(())
}