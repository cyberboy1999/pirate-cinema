"use client";

import {useMemo,useState} from "react";
import {Check,CircleNotch,FilmStrip,GearSix,House,LinkSimple,Magnet,MagnifyingGlass,Play,Plus,SlidersHorizontal,X} from "@phosphor-icons/react";
import styles from "./mobile.module.css";

type Media={id:number;title:string;year:number;kind:"movie"|"series";files:string[];progress:number;time:string;tone:string};
const MEDIA:Media[]=[
  {id:1,title:"Путь домой",year:2024,kind:"movie",files:["Путь домой (2024) 1080p.mkv"],progress:78,time:"1:42:17 / 2:18:03",tone:"moon"},
  {id:2,title:"Город теней",year:2023,kind:"series",files:["Сезон 1 · Серия 1.mkv","Сезон 1 · Серия 2.mkv","Сезон 1 · Серия 3.mkv"],progress:19,time:"23:11 / 1:58:20",tone:"city"},
  {id:3,title:"Последний горизонт",year:2022,kind:"movie",files:["Последний горизонт 4K.mkv"],progress:0,time:"Не просмотрено",tone:"sun"},
  {id:4,title:"Тихие люди",year:2024,kind:"series",files:["01. Возвращение.mkv","02. Дом у озера.mkv","03. После полуночи.mkv"],progress:0,time:"Не просмотрено",tone:"forest"},
  {id:5,title:"Глубже",year:2023,kind:"movie",files:["Глубже 1080p.mkv"],progress:49,time:"47:35 / 1:36:12",tone:"ocean"},
];

export default function MobilePrototype(){
  const [section,setSection]=useState<"home"|"library"|"settings">("home"),[mode,setMode]=useState<"library"|"search">("library"),[filter,setFilter]=useState("all"),[query,setQuery]=useState(""),[selected,setSelected]=useState<Media|null>(null),[file,setFile]=useState(""),[player,setPlayer]=useState<"mpv"|"system">("mpv"),[magnetOpen,setMagnetOpen]=useState(false),[toast,setToast]=useState("");
  const visible=useMemo(()=>MEDIA.filter(item=>(filter==="all"||filter==="unseen"&&item.progress===0||item.kind===filter)&&item.title.toLowerCase().includes(query.toLowerCase())),[filter,query]);
  function open(item:Media){setSelected(item);setFile(item.files[0]);setPlayer("mpv")}
  function launch(){setSelected(null);setToast(player==="mpv"?"Передаём поток в mpv-android":"Открываем системный выбор плеера");setTimeout(()=>setToast(""),2600)}
  return <main className={styles.stage}><section className={styles.phone}>
    <header className={styles.header}><div><span>12:30</span><h1>Pirate Cinema</h1></div><button className={styles.status}><i/>127.0.0.1:8090 · Доступен</button></header>
    {section!=="settings"?<>
      <button className={styles.server}><span><Check size={22} weight="bold"/></span><div><strong>TorrServer готов</strong><small>Можно искать и смотреть фильмы</small></div></button>
      <div className={styles.segment}><button className={mode==="library"?styles.active:""} onClick={()=>setMode("library")}>Медиатека</button><button className={mode==="search"?styles.active:""} onClick={()=>setMode("search")}>Поиск</button></div>
      <label className={styles.search}><MagnifyingGlass size={23}/><input value={query} onChange={event=>setQuery(event.target.value)} placeholder={mode==="search"?"Название фильма или сериала":"Название в медиатеке"}/>{query&&<button onClick={()=>setQuery("")}><X size={18}/></button>}</label>
      {mode==="library"?<>
        <div className={styles.filters}><button className={filter==="all"?styles.chosen:""} onClick={()=>setFilter("all")}>Все</button><button className={filter==="movie"?styles.chosen:""} onClick={()=>setFilter("movie")}>Фильмы</button><button className={filter==="series"?styles.chosen:""} onClick={()=>setFilter("series")}>Сериалы</button><button className={filter==="unseen"?styles.chosen:""} onClick={()=>setFilter("unseen")}>Не просмотрено</button></div>
        <div className={styles.list}>{visible.map(item=><article key={item.id}><button className={`${styles.thumb} ${styles[item.tone]}`} onClick={()=>open(item)} aria-label={`Открыть ${item.title}`}><Play size={22} weight="fill"/></button><button className={styles.info} onClick={()=>open(item)}><strong>{item.title}</strong><small>{item.year} · {item.files.length} {item.files.length===1?"файл":"файла"}</small>{item.progress>0&&<span className={styles.progress}><i style={{width:`${item.progress}%`}}/></span>}<small>{item.time}{item.progress>0?` · ${item.progress}%`:""}</small></button><button className={styles.play} onClick={()=>open(item)} aria-label={`Воспроизвести ${item.title}`}><Play size={19} weight="fill"/></button></article>)}</div>
      </>:<SearchResults query={query} onAdd={()=>{setMagnetOpen(true);setToast("")}}/>}
      <button className={styles.fab} onClick={()=>setMagnetOpen(true)} aria-label="Добавить magnet-ссылку"><Plus size={27}/></button>
    </>:<Settings/>}
    <nav className={styles.nav}><button className={section==="home"?styles.current:""} onClick={()=>{setSection("home");setMode("library")}}><House size={24} weight={section==="home"?"fill":"regular"}/>Главная</button><button className={section==="library"?styles.current:""} onClick={()=>{setSection("library");setMode("library")}}><FilmStrip size={24}/>Медиатека</button><button className={section==="settings"?styles.current:""} onClick={()=>setSection("settings")}><GearSix size={24}/>Настройки</button></nav>
    {selected&&<div className={styles.overlay}><button className={styles.backdrop} onClick={()=>setSelected(null)} aria-label="Закрыть выбор плеера"/><section className={styles.sheet}><button className={styles.close} onClick={()=>setSelected(null)}><X size={22}/></button><span className={styles.eyebrow}>ВОСПРОИЗВЕДЕНИЕ</span><h2>{selected.title}</h2><label>Файл<select value={file} onChange={event=>setFile(event.target.value)}>{selected.files.map(value=><option key={value}>{value}</option>)}</select></label><div className={styles.players}><button className={player==="mpv"?styles.selectedPlayer:""} onClick={()=>setPlayer("mpv")}><span><Play size={20} weight="fill"/></span><div><strong>mpv-android</strong><small>Рекомендуется · вернёт позицию просмотра</small></div>{player==="mpv"&&<Check size={20}/>}</button><button className={player==="system"?styles.selectedPlayer:""} onClick={()=>setPlayer("system")}><span><SlidersHorizontal size={20}/></span><div><strong>Открыть с помощью</strong><small>Выбрать другой установленный плеер</small></div>{player==="system"&&<Check size={20}/>}</button></div><button className={styles.primary} onClick={launch}><Play size={19} weight="fill"/>Воспроизвести</button></section></div>}
    {magnetOpen&&<div className={styles.overlay}><button className={styles.backdrop} onClick={()=>setMagnetOpen(false)} aria-label="Закрыть добавление раздачи"/><section className={styles.sheet}><button className={styles.close} onClick={()=>setMagnetOpen(false)}><X size={22}/></button><span className={styles.eyebrow}>TORRSERVER</span><h2>Добавить раздачу</h2><label>Magnet-ссылка<textarea placeholder="magnet:?xt=urn:btih:…"/></label><button className={styles.primary} onClick={()=>{setMagnetOpen(false);setToast("Раздача отправлена в TorrServer")}}><Magnet size={19}/>Добавить</button></section></div>}
    {toast&&<div className={styles.toast}><Check size={18}/>{toast}</div>}
  </section></main>
}

function SearchResults({query,onAdd}:{query:string;onAdd:()=>void}){return <div className={styles.results}><p>{query?`Результаты TorrServer для «${query}»`:"Введите название — поиск выполнит TorrServer"}</p>{query&&["1080p · 8.4 ГБ","2160p · 31.7 ГБ","Сезон 1 · 12 серий"].map((meta,index)=><article key={meta}><div><strong>{query}</strong><small>{meta} · {index+4} источника</small></div><button onClick={onAdd}><Plus size={18}/>Добавить</button></article>)}</div>}

function Settings(){const [checking,setChecking]=useState(false);return <section className={styles.settings}><span className={styles.eyebrow}>НАСТРОЙКИ</span><h2>Подключение</h2><label>Адрес TorrServer<input defaultValue="http://127.0.0.1:8090"/></label><button className={styles.check} onClick={()=>{setChecking(true);setTimeout(()=>setChecking(false),900)}}>{checking?<CircleNotch className={styles.spin} size={19}/>:<LinkSimple size={19}/>}Проверить подключение</button><div className={styles.settingCard}><span><Play size={21} weight="fill"/></span><div><strong>mpv-android</strong><small>Рекомендуемый внешний плеер</small></div><Check size={20}/></div><p>Если mpv-android недоступен, приложение покажет системное меню выбора плеера.</p></section>}
