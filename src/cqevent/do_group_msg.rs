use crate::{cqapi::*, redlang::{RedLang}, mytool::json_to_cq_str, read_config};

use super::{is_key_match, get_script_info, set_normal_message_info};

fn msg_id_map_insert(user_id:String,group_id:String,message_id:String) ->Result<(), Box<dyn std::error::Error>> {
    let flag = user_id + &group_id;
    let mut mp = crate::G_MSG_ID_MAP.write()?;
    if mp.contains_key(&flag) {
        let v = mp.get_mut(&flag).unwrap();
        v.insert(0, message_id.to_string());
        if v.len() > 20 {
            v.pop();
        }
    }else{
        let v = vec![message_id.to_string()];
        mp.insert(flag, v);
    }
    Ok(())
}

fn do_script(rl:&mut RedLang,code:&str) -> Result<(), Box<dyn std::error::Error>>{
    let out_str = rl.parse(code)?;
    let group_id = rl.get_exmap("群ID")?.parse::<i32>()?;
    if out_str != "" {
        let send_json = serde_json::json!({
            "action":"send_group_msg",
            "params":{
                "group_id": group_id,
                "message":out_str
            }
        });
        let ret_str = cq_call_api(&send_json.to_string())?;
        let ret_json:serde_json::Value = serde_json::from_str(&ret_str)?;
        let retcode = ret_json.get("retcode").ok_or("retcode not found")?.as_i64().ok_or("retcode not int")?;
        if retcode != 0 {
            cq_add_log_w(&ret_str).unwrap();
        }else {
            let data = ret_json.get("data").ok_or("data not found")?;
            let message_id = data.get("message_id").ok_or("message_id not found")?.as_i64().ok_or("retcode not int")?;
            let group_id_str = group_id.to_string();
            let self_id = rl.get_exmap("机器人ID")?;
            msg_id_map_insert(self_id.to_string(),group_id_str,message_id.to_string())?;
        }
    }
    Ok(())
}

fn do_redlang(root: &serde_json::Value) -> Result<(), Box<dyn std::error::Error>>{
    let msg = json_to_cq_str(&root)?;
    let script_json = read_config()?;
    let mut is_set_msg_id_map = false;
    for i in 0..script_json.as_array().ok_or("script.json文件不是数组格式")?.len(){
        let (keyword,cffs,code,ppfs) = get_script_info(&script_json[i])?;
        let mut rl = RedLang::new();
        if cffs == "群聊触发" || cffs == "群、私聊触发"{
            rl.set_exmap("内容", &msg)?;
            set_normal_message_info(&mut rl, root)?;
            if is_set_msg_id_map == false {
                is_set_msg_id_map = true;
                let user_id = rl.get_exmap("发送者ID")?;
                let group_id = rl.get_exmap("群ID")?;
                let message_id = rl.get_exmap("消息ID")?;
                msg_id_map_insert(user_id.to_string(),group_id.to_string(),message_id.to_string())?;
            }
            {
                let sender = root.get("sender").ok_or("sender not exists")?;
                {
                    let role_js = sender.get("role").ok_or("role in sender not exists")?;
                    let role = role_js.as_str().ok_or("role in sender not str")?;
                    let role_str = match role {
                        "owner" => "群主",
                        "admin" => "管理",
                        &_ => "群员"
                    };
                    rl.set_exmap("发送者权限", role_str)?;
                }
                if let Some(js_v) = sender.get("card") {
                    rl.set_exmap("发送者名片", js_v.as_str().unwrap_or(""))?;
                }
                if let Some(js_v) = sender.get("title") {
                    rl.set_exmap("发送者专属头衔", js_v.as_str().unwrap_or(""))?;
                }
            }
            if is_key_match(&mut rl,&ppfs,keyword,&msg)? {
                do_script(&mut rl,code)?;
            }    
        }
    }
    Ok(())
}

// 处理群聊事件
pub fn do_group_msg(root: &serde_json::Value) -> Result<i32, Box<dyn std::error::Error>> {
 
    if let Err(e) = do_redlang(&root) {
        cq_add_log_w(format!("err in do_group_msg:do_redlang:{}", e.to_string()).as_str()).unwrap();
    }
    Ok(0)
}
