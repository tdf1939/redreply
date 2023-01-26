const { createApp } = Vue
            createApp({
                data() {
                    return {
                        // 用于显示
                        script_name:"名字",
                        script_description:"介绍",
                        script_keyword:"关键词",
                        script_ppfs:"匹配方式",
                        script_cffs:"触发方式",
                        script_content:"脚本内容",
                        select_name_index: -1,
                        // 用于记录所有数据
                        codes: "正在加载内容...",
                        pkg_codes : {"默认包":[]},
                        select_pkg_name:"默认包"
                    }
                },
                mounted () {
                    axios
                    .get("/get_all_pkg_name")
                    .then(
                    res => {
                        let ret = res.data["data"];
                        // console.log(ret)
                        this.pkg_codes["默认包"] = []
                        for(let i = 0;i < ret.length;++i) {
                            this.pkg_codes[ret[i]] = []
                        }
                        axios
                        .get("/get_code")
                        .then(
                        res => {
                            this.codes = res.data["data"]
                            for(let i = 0;i < this.codes.length;++i) {
                                let pkg_name = this.codes[i]["pkg_name"]
                                if(pkg_name == undefined) {
                                    pkg_name = "默认包"
                                }
                                if(this.pkg_codes[pkg_name] == undefined) {
                                    this.pkg_codes[pkg_name] = []
                                }
                                this.pkg_codes[pkg_name].push(this.codes[i])
                            }
                            // console.log(this.pkg_codes)
                        })
                        .catch(function (error) {
                            console.log(error);
                        });
                    })
                    .catch(function (error) {
                        console.log(error);
                    });
                    
                },
                methods: {
                    select_name_index_change(new_select) {
                        // 切换新数据
                        if(new_select != -1){
                            this.script_name = this.pkg_codes[this.select_pkg_name][new_select]["name"]
                            this.script_description = this.pkg_codes[this.select_pkg_name][new_select]["description"]
                            this.script_keyword = this.pkg_codes[this.select_pkg_name][new_select]["content"]["关键词"]
                            this.script_ppfs = this.pkg_codes[this.select_pkg_name][new_select]["content"]["匹配方式"]
                            this.script_cffs = this.pkg_codes[this.select_pkg_name][new_select]["content"]["触发方式"]
                            this.script_content = this.pkg_codes[this.select_pkg_name][new_select]["content"]["code"]
                        }else{
                            this.script_name = ""
                            this.script_description = ""
                            this.script_keyword = ""
                            this.script_ppfs = ""
                            this.script_cffs = ""
                            this.script_content = ""
                        }
                    },
                    // 缓存旧数据
                    save_cache(old_select) {
                        if(old_select != -1){
                            this.pkg_codes[this.select_pkg_name][old_select]["pkg_name"] = this.select_pkg_name;
                            this.pkg_codes[this.select_pkg_name][old_select]["name"] = this.script_name;
                            this.pkg_codes[this.select_pkg_name][old_select]["description"] = this.script_description;
                            this.pkg_codes[this.select_pkg_name][old_select]["content"]["关键词"] = this.script_keyword;
                            this.pkg_codes[this.select_pkg_name][old_select]["content"]["匹配方式"] = this.script_ppfs;
                            this.pkg_codes[this.select_pkg_name][old_select]["content"]["触发方式"] = this.script_cffs;
                            this.pkg_codes[this.select_pkg_name][old_select]["content"]["code"] = this.script_content;
                        }
                    },
                    save_code() {
                        this.save_cache(this.select_name_index);
                        let code = []
                        
                        for(let k in this.pkg_codes) {
                            // console.log(k)
                            // console.log(this.pkg_codes[k])
                            
                            if(k == "默认包"){
                                for(let it in this.pkg_codes[k])
                                {
                                    let kk = JSON.parse(JSON.stringify(this.pkg_codes[k][it]))
                                    delete kk["pkg_name"]
                                    code.push(kk)
                                }
                            }else
                            {
                                for(let it in this.pkg_codes[k])
                                {
                                    code.push(this.pkg_codes[k][it])
                                }
                            }
                            
                        }
                        axios
                        .post("/set_code",code)
                        .then(function (res){
                            alert("保存成功")
                        })
                        .catch(function (error) {
                            console.log(error);
                            alert("保存失败")
                        });
                        
                    },
                    add_code() {
                        this.save_cache(this.select_name_index);
                        this.pkg_codes[this.select_pkg_name].push({"pkg_name":this.select_pkg_name,"name":"code_name","description":"code_description","content":{"关键词":"222","触发方式":"群聊触发","匹配方式":"完全匹配","code":"hello"}})
                        this.select_name_index = this.pkg_codes[this.select_pkg_name].length - 1;
                        this.select_name_index_change(this.select_name_index)
                    },
                    del_code() {
                        if(this.select_name_index != -1){
                            this.pkg_codes[this.select_pkg_name].splice(this.select_name_index,1);
                            this.select_name_index = -1
                        }
                    },
                    help_web() {
                        window.open("/readme.html", "_blank");
                    },
                    watch_log() {
                        window.open("/watchlog.html", "_blank");
                    },
                    quit_redreply() {
                        setTimeout(function(){
                            location.reload();
                        },1000);
                        axios.get("/close")
                    }
                }
            }).mount('#app')