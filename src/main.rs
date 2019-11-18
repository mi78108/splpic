use sqlite::State;
use chrono::{NaiveDateTime, Local};
use std::{fs, thread, env};
use std::fs::{read_dir, read, File, create_dir};
use std::process::exit;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Sender, channel, Receiver};
use std::sync::{Arc, Mutex};
use std::ops::Add;
use std::io::{Seek, SeekFrom, Read, Write};
use regex::Regex;

use lazy_static::lazy_static;
use std::collections::HashMap;
use core::iter;

lazy_static! {
    static ref CNF:HashMap<String,String> = get_init();
}

fn main() {
    let mut tp = ThreadPool::new(CNF["tpn"].parse().unwrap());
    match read_dir(&CNF["source"]) {
        Ok(d) => {
            for f in d {
                if f.as_ref().unwrap().file_type().unwrap().is_dir() && f.as_ref().unwrap().file_name().to_str().unwrap().starts_with("datadir") {
                    for s in read_dir(f.unwrap().path()).unwrap() {
                        if s.as_ref().unwrap().file_type().unwrap().is_file() && s.as_ref().unwrap().file_name().to_str().unwrap().starts_with("pic_segment_db_index") {
                            tp.add(Box::new(|| {
                                do_this(s.unwrap().path());
                            }));
                        }
                    }
                }
            }
        }
        Err(_) => {
            eprintln!("文件夹打开失败，请检查文件夹是否存在")
        }
    }
    ThreadPool::join(tp);
}

/// 打开一个特定数据库，查询有效数据，分割文件并按规则保存
fn do_this(p: PathBuf) {
    let r = sqlite::open(&p).expect(format!("数据库 {} 打开失败", &p.to_str().unwrap()).as_str());
    let mut sql = String::from("SELECT t.segment_id,t.start_time,t.start_offset,t.end_offset FROM pic_segment_idx_tb t where t.pic_type != 0 ");
    if let (Some(s), Some(e)) = (CNF.get("us"), CNF.get("ue")) {
        sql.push_str(format!(" and t.start_time >= {} and t.end_time <= {} ", s, e).as_str())
    }

    let mut r = r.prepare(sql.as_str()).unwrap();
    match fs::File::open(Path::new(&p.as_path().to_str().unwrap().to_string().replace("pic_segment_db_index", "hiv").add(".pic"))) {
        Ok(mut f) => {
            let mut tp = ThreadPool::new(CNF["tpn"].parse().unwrap());
            while let State::Row = r.next().unwrap() {
                let id = r.read::<String>(0).expect("Id 错误");
                let tm = r.read::<i64>(1).unwrap();
                //eprintln!("Time: {}", NaiveDateTime::from_timestamp(tm, 0).format("%Y%m%d_%H%M%S_%s"));
                let so = r.read::<i64>(2).unwrap();
                let eo = r.read::<i64>(3).unwrap();
                let tm = NaiveDateTime::from_timestamp(tm, 0);
                eprintln!("pkId = {} | time = {} | so = {} | eo = {}", id, tm, so, eo);

                let mut buf = iter::repeat(0u8).take((eo - so) as usize).collect::<Vec<u8>>();
                f.seek(SeekFrom::Start(so as u64)).expect("移动文件指针错误");
                if f.read(&mut buf).unwrap() == buf.len() && buf.len() > 0 {
                    eprintln!("Len :{}\n", buf.len());
                    tp.add(Box::new(move || {
                        let mut dir = PathBuf::from(&CNF["target"]);
                        if CNF.contains_key("dir") {
                            dir = dir.join(tm.format(&CNF["dir"]).to_string())
                        }
                        let dir = dir.join(format!("{}_{}.jpg", &CNF["idn"], tm.format("%Y%m%d_%H%M%S_%s"))).to_string_lossy().to_string();
                        match File::create(dir.as_str()) {
                            Ok(mut tf) => {
                                tf.write(&buf).unwrap();
                                match tf.flush() {
                                    Ok(_) => println!("{}", dir.as_str()),
                                    Err(_) => eprintln!("{} 写入失败", dir.as_str())
                                }
                            }
                            Err(_) => {
                                eprintln!("存取文件打开错误 path: {}", dir.as_str());
                            }
                        }
                    }));
                }
            }
            ThreadPool::join(tp);
        }
        Err(_) => {
            eprintln!("资源 文件不存在")
        }
    };
}

/// 初始化参数和环境
fn get_init() -> HashMap<String, String> {
    let mut args: HashMap<String, String> = HashMap::new();
    args.insert("tpn".to_string(), "3".to_string());
    env::args().into_iter().for_each(|v| {
        if v.len() >= 2 {
            match v[0..2].to_lowercase().as_str() {
                "-s" => args.insert("source".to_string(), v.gsub("-[sS]", "")),
                "-t" => args.insert("target".to_string(), v.gsub("-[tT]", "")),
                "-n" => args.insert("tpn".to_string(), v.gsub("-[nN]", "")),
                "-r" => {
                    let t = v.gsub("-[rR]", "");
                    if t.is_empty() {
                        args.insert("dir".to_string(), "%Y%m%d".to_string())
                    } else {
                        args.insert("dir".to_string(), t)
                    }
                }
                "-u" => {
                    let t = v.gsub("-[uU]", "");
                    let t = t.split("-").collect::<Vec<&str>>();
                    match t.len() {
                        1 => {
                            args.insert("us".to_string(), t[0].to_string());
                            args.insert("ue".to_string(), Local::now().naive_local().timestamp().to_string());
                        }
                        2 => {
                            args.insert("us".to_string(), t[0].to_string());
                            args.insert("ue".to_string(), t[1].to_string());
                        }
                        _ => {}
                    }
                    None
                }
                "-d" => {
                    let t = v.gsub("-[dD]", "");
                    let t = t.split("-").collect::<Vec<&str>>();
                    match t.len() {
                        1 => {
                            args.insert("us".to_string(), NaiveDateTime::parse_from_str(t[0], "%Y%m%d%H%M%S").expect("时间格式错误").timestamp().to_string());
                            args.insert("ue".to_string(), Local::now().naive_local().timestamp().to_string());
                        }
                        2 => {
                            args.insert("us".to_string(), NaiveDateTime::parse_from_str(t[0], "%Y%m%d%H%M%S").expect("时间格式错误").timestamp().to_string());
                            args.insert("ue".to_string(), NaiveDateTime::parse_from_str(t[1], "%Y%m%d%H%M%S").expect("时间格式错误").timestamp().to_string());
                        }
                        _ => {
                            eprintln!("时间参数错误");
                            exit(1)
                        }
                    }
                    None
                }
                _ => None
            };
        }
    });

//    if args.len() < 2 && env::args().len() > 2 {
//        args.insert("source".to_string(), env::args().collect::<Vec<String>>()[1].to_string());
//        args.insert("target".to_string(), env::args().collect::<Vec<String>>()[2].to_string());
//        if env::args().len() == 4 {
//            args.insert("tpn".to_string(), env::args().collect::<Vec<String>>()[3].to_string());
//        }
//    }
    if args.len() < 3 {
        eprintln!("Command -[sS]source_dir -[tT]target_dir [-n]thread_number_def_5");
        eprintln!("参数错误");
        exit(1);
    }

    //参数长度判断
    args.values().for_each(|v| {
        if v.is_empty() {
            eprintln!("参数不能为空");
            exit(1);
        }
    });

    //检查目标目录
    if args.contains_key("target") {
        let dir = Path::new(args.get("target").unwrap());
        if dir.parent().unwrap().exists() {
            if !dir.exists() {
                create_dir(dir).unwrap()
            }
        } else {
            eprintln!("请指定正确的父路径")
        }
    } else {
        eprintln!("存储路径 为指定");
        exit(1)
    }

    //创建存储目录子文件夹
    if args.contains_key("dir") && args.contains_key("target") && args.contains_key("us") {
        ((args.get("us").unwrap().parse::<usize>().unwrap())..(args.get("ue").unwrap().parse::<usize>().unwrap())).step_by(60 * 60 * 24).for_each(|i| {
            let dir = Path::new(args.get("target").unwrap()).join(NaiveDateTime::parse_from_str(i.to_string().as_str(), "%s").unwrap().format(args.get("dir").unwrap()).to_string());
            match dir.exists() {
                true => {
                    if dir.is_dir() {
                        eprintln!("Create  Dir {}", dir.as_path().to_str().unwrap());
                    } else {
                        eprintln!("Creating Dir {} Error File exits", &dir.as_path().to_str().unwrap());
                        exit(1);
                    }
                }
                false => {
                    if let Ok(_) = create_dir(&dir) {
                        eprintln!("Created Dir {}", &dir.as_path().to_str().unwrap());
                    } else {
                        eprintln!("Creating Dir {} Error", &dir.as_path().to_str().unwrap());
                        exit(1);
                    }
                }
            }
        });
    }

    match read(Path::new(&args["source"]).join("info.bin")) {
        Ok(s) => {
            let r = String::from_utf8_lossy(s.iter().take_while(|v| **v > 0).map(|v| *v).collect::<Vec<u8>>().as_slice()).to_string();
            let r = r.split("-").collect::<Vec<&str>>();
            if r.len() >= 3 {
                //r[2].to_string()
                args.insert("idn".to_string(), r[2].to_string());
            } else {
                eprintln!("设备序号获取有误");
                exit(2);
            }
        }
        Err(_) => {
            eprintln!("设备信息打开失败 检查目录文件是否正确");
            exit(3);
        }
    };
    eprintln!("Thread count {}", (&args)["tpn"]);
    args
}


pub trait FnBox {
    fn call_box(self: Box<Self>);
}

impl<F: FnOnce()> FnBox for F {
    fn call_box(self: Box<F>) {
        (*self)()
    }
}

pub type Task = Box<dyn FnBox + Send>;

struct ThreadPool {
    count: Option<isize>,
    tx: Option<Sender<Task>>,
    crx: Option<Receiver<String>>,
    handlers: Option<Vec<thread::JoinHandle<()>>>,
}

impl ThreadPool {
    pub fn new(size: usize) -> ThreadPool {
        let (tx, rx) = channel::<Task>();
        let (ctx, crx) = channel::<String>();

        let mut handlers = Vec::new();

        let arx = Arc::new(Mutex::new(rx));
        let ctx = Arc::new(Mutex::new(ctx));

        let get_task = |l: &Arc<Mutex<Receiver<Task>>>| {
            match l.lock().expect("A E").recv() {
                Ok(s) => Some(s),
                Err(_) => None
            }
        };
        for _ in 0..size {
            let arx = arx.clone();
            let ctx = ctx.clone();
            let handle = thread::spawn(move || {
                while let Some(task) = get_task(&arx) {
                    //   eprintln!("Thread {} Started", i);
                    task.call_box();
                    ctx.lock().unwrap().send("ok".to_string()).unwrap();
                    // eprintln!("Thread {} Finished\n", i);
                }
            });
            handlers.push(handle);
            //  eprintln!("Thread {} Created", i);
        }

        ThreadPool {
            count: Some(0 as isize),
            tx: Some(tx),
            crx: Some(crx),
            handlers: Some(handlers),
        }
    }

    pub fn add(&mut self, task: Task) {
        // eprintln!("Pool Has {}", self.count.unwrap());
        self.count = Some(self.count.unwrap() + 1);
        self.tx.as_ref().unwrap().send(task).unwrap()
    }

    pub fn join(mut tp: ThreadPool) {
        // eprintln!("Main Done");
        if tp.count.unwrap() > 0 {
            for _ in tp.crx.expect("C E>>>>>>>>>>>>>>>>>") {
                tp.count = Some(tp.count.unwrap() - 1);
                //    eprintln!("Done With {} Stil Has {}", i, tp.count.unwrap());
                if tp.count.unwrap() <= 0 {
                    eprintln!("Thread Pool All Done");
                    break;
                    //  thread::sleep(Duration::new(0, 500));
                }
            }
        }
    }
}

trait StringExt {
    fn gsub(&self, reg: &str, pat: &str) -> String;
}

impl StringExt for String {
    fn gsub(&self, reg: &str, pat: &str) -> String {
        Regex::new(reg).unwrap().replace_all(self, pat).to_string()
    }
}