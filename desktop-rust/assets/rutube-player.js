window.startRutube = function (config) {
    window.rutubeFlush?.();
    window.rutubePlayer?.destroy(false);
    const send = message => dioxus.send({status:message});
    let sequence = 0;
    const pending = new Map();
    let stream = null;
    let currentId = config.id;
    let changing = false;
    let resumeAfterSwitch = false;
    let restorePosition = Number(config.position) || 0;
    let played = false;
    let lastSave = 0;
    let preferredHeight = Number(config.quality) || 0;
    let autoNext = config.autoNext !== false;
    let retrying = false, retries = 0, retryTimer = null, recoveryPosition = 0, recoveryPlaying = false;
    const report = force => {
        const position = art.currentTime, duration = art.duration;
        if (!played || restorePosition > 0 || !Number.isFinite(position) || !Number.isFinite(duration) || duration <= 0) return;
        if (!force && Date.now() - lastSave < 3000) return;
        lastSave = Date.now();
        dioxus.send({progress:currentId,position,duration});
    };
    const queue = config.queue || [];
    const escapeHtml = text => String(text).replace(/[&<>"']/g, char => ({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[char]));
    const visible = () => !document.hidden && window.__pirateCinemaVisible !== false;
    const choose = id => {
        if (changing || id === currentId || !queue.some(item => item.id === id)) return;
        changing = true;
        clearTimeout(retryTimer);
        art.pause();
        dioxus.send({episode:id});
    };
    const retry = () => {
        if (retrying || changing || !visible()) return;
        retrying = true;
        recoveryPosition = restorePosition || (Number.isFinite(art.currentTime) ? art.currentTime : 0);
        recoveryPlaying = resumeAfterSwitch || !art.paused;
        art.pause();
        send('Восстанавливаем поток…');
        dioxus.send({retry:currentId});
    };
    window.rutubeReply = (token, id, data, error) => {
        if (token !== config.token) return;
        const callback = pending.get(id);
        if (!callback) return;
        pending.delete(id);
        callback(data, error);
    };
    class NativeLoader extends Hls.DefaultConfig.loader {
        load(context, options, callbacks) {
            this.context = context;
            this.stats.loading.start = performance.now();
            const id = ++sequence;
            this.requestId = id;
            const timer = setTimeout(() => {
                pending.delete(id);
                callbacks.onTimeout(this.stats, context, null);
            }, 25000);
            this.requestTimer = timer;
            pending.set(id, (data, error) => {
                clearTimeout(timer);
                if (error) { callbacks.onError({code:0,text:error}, context, null, this.stats); return; }
                const bytes = Uint8Array.from(atob(data), c => c.charCodeAt(0));
                const body = context.responseType === 'arraybuffer' ? bytes.buffer : new TextDecoder().decode(bytes);
                this.stats.loaded = this.stats.total = bytes.length;
                this.stats.loading.first = this.stats.loading.end = performance.now();
                callbacks.onSuccess({url:context.url,data:body,code:200}, this.stats, context, null);
            });
            const hasRange = Number.isFinite(context.rangeEnd) && context.rangeEnd > (context.rangeStart || 0);
            dioxus.send({id,url:context.url,start:hasRange ? (context.rangeStart || 0) : null,end:hasRange ? context.rangeEnd : null});
        }
        abort() {
            clearTimeout(this.requestTimer);
            pending.delete(this.requestId);
            super.abort();
        }
        destroy() { this.abort(); super.destroy(); }
    }
    const art = new Artplayer({
        container: '#rutube-player', url: config.url, type: 'm3u8',
        lang: 'ru', theme: '#398bff', autoplay: false,
        i18n: {ru: {'Play':'Воспроизвести','Pause':'Пауза','Show Setting':'Настройки','Hide Setting':'Скрыть настройки','Fullscreen':'Полный экран','Exit Fullscreen':'Выйти из полного экрана','Web Fullscreen':'На всё окно','Exit Web Fullscreen':'Вернуть размер','PIP Mode':'Картинка в картинке','Play Speed':'Скорость','Aspect Ratio':'Пропорции','Normal':'Обычная','Volume':'Громкость','Video Load Failed':'Не удалось загрузить видео'}},
        fullscreen: true, fullscreenWeb: true, pip: true,
        setting: true, playbackRate: true, aspectRatio: true,
        customType: {
            m3u8(video, url, player) {
                if (Hls.isSupported()) {
                    const hls = new Hls({loader: NativeLoader, maxBufferLength: 30, maxMaxBufferLength: 60});
                    stream = hls;
                    let selectedQuality = -1;
                    let qualityAdded = false;
                    const updateQuality = () => {
                        const variants = hls.levels.map((level, index) => ({
                            html: Number(level.height) > 0 ? `${Number(level.height)}p` : `Вариант ${index + 1}`,
                            value: index, height: Number(level.height) || 0,
                            bitrate: Number(level.bitrate) || 0
                        })).sort((a, b) => b.height - a.height || b.bitrate - a.bitrate);
                        for (const item of variants) {
                            if (variants.filter(other => other.height === item.height).length > 1) {
                                item.html += ` · ${Math.round(item.bitrate / 1000)} кбит/с`;
                            }
                        }
                        selectedQuality = preferredHeight ? hls.levels.findIndex(level=>Number(level.height)===preferredHeight) : -1;
                        hls.nextLevel = selectedQuality;
                        const selector = [{html:'Авто', value:-1}, ...variants]
                            .map(item => ({...item, default:item.value === selectedQuality}));
                        player.setting[qualityAdded ? 'update' : 'add']({
                            name:'rutube-quality', html:'Качество', width:250,
                            tooltip:selector.find(item => item.default)?.html || 'Авто', selector,
                            onSelect(item) {
                                if (!Number.isInteger(item.value) || item.value < -1 || item.value >= hls.levels.length) return;
                                selectedQuality = item.value;
                                preferredHeight = item.value < 0 ? 0 : Number(hls.levels[item.value].height) || 0;
                                dioxus.send({preference:'quality',value:preferredHeight});
                                // Switch the HLS level, not the player URL: retain position, pause and fullscreen.
                                hls.nextLevel = item.value;
                                return item.html;
                            }
                        });
                        qualityAdded = true;
                    };
                    hls.loadSource(url); hls.attachMedia(video);
                    player.on('destroy', () => hls.destroy());
                    hls.on(Hls.Events.MANIFEST_PARSED, () => {
                        updateQuality();
                        send('Готово. Нажмите воспроизведение.');
                        if (resumeAfterSwitch) {
                            resumeAfterSwitch = false;
                            if (visible()) player.play().catch(() => {});
                        }
                    });
                    hls.on(Hls.Events.LEVELS_UPDATED, updateQuality);
                    hls.on(Hls.Events.ERROR, (_, error) => {
                        if (error.fatal) {
                            hls.stopLoad();
                            art.controls['rutube-retry'].style.display = '';
                            send('Поток прервался. Доступна кнопка «Повторить».');
                            if (retries < 2 && visible() && !retrying) {
                                retries++;
                                clearTimeout(retryTimer);
                                retryTimer = setTimeout(retry, retries * 2000);
                            }
                        }
                    });
                } else if (video.canPlayType('application/vnd.apple.mpegurl')) {
                    video.src = url;
                } else send('HLS не поддерживается этим WebView.');
            }
        }
    });
    art.controls.add({name:'rutube-retry',position:'right',html:'Повторить',tooltip:'Восстановить поток с текущей позиции',style:{display:'none'},click:retry});
    window.rutubeRetryFailed = () => { retrying = false; send('Поток пока недоступен. Нажмите «Повторить».'); };
    window.rutubeRecover = url => {
        if (!retrying || changing) return;
        retrying = false;
        restorePosition = recoveryPosition;
        resumeAfterSwitch = recoveryPlaying;
        if (stream) { stream.recoverMediaError(); stream.loadSource(url); }
        else art.url = url;
    };
    if (queue.length) {
        const info = queue.map((item,index) => {
            const match = item.title.match(/(?:S(\d+)\s*E(\d+)|(\d+)\s*сезон\s*(\d+)\s*сери)/i);
            return {...item,season:match ? Number(match[1] || match[3]) : 0,label:match ? `Серия ${Number(match[2] || match[4])}` : `Выпуск ${index+1}`};
        });
        let season = info.find(item=>item.id===currentId)?.season || 0;
        const seasons = [...new Set(info.map(item=>item.season))];
        const updateEpisodes = () => art.setting[art.__episodesAdded ? 'update' : 'add']({
            name:'rutube-episodes', html:'Серия', width:220,
            tooltip:info.find(item=>item.id===currentId)?.label,
            selector:info.filter(item=>item.season===season).map(item => ({html:escapeHtml(item.label), value:item.id, default:item.id === currentId})),
            onSelect(item) { choose(item.value); return item.html; }
        });
        updateEpisodes();
        art.__episodesAdded = true;
        let seasonAdded=false;
        const updateSeason=()=>{
            art.setting[seasonAdded ? 'update' : 'add']({name:'rutube-season',html:'Сезон',width:220,tooltip:season ? `Сезон ${season}` : 'Без сезона',selector:seasons.map(value=>({html:value ? `Сезон ${value}` : 'Без сезона',value,default:value===season})),onSelect(item){ if (!seasons.includes(item.value)) return; season=item.value; updateEpisodes();return item.html; }});
            seasonAdded=true;
        };
        updateSeason();
        art.setting.add({name:'rutube-auto-next',html:'Автопереход',switch:autoNext,onSwitch(item){autoNext=!item.switch;dioxus.send({preference:'auto_next',value:autoNext ? 1 : 0});return autoNext;}});
        const next = () => { const index=queue.findIndex(item=>item.id===currentId); return index>=0 ? queue[index+1] : null; };
        art.controls.add({name:'rutube-next',position:'right',html:'Следующая серия →',style:{display:'none'},click:()=>{const item=next();if(item)choose(item.id);}});
        art.on('video:timeupdate', () => {
            const remaining=art.duration-art.currentTime;
            art.controls['rutube-next'].style.display = next() && remaining>0 && remaining<=120 ? '' : 'none';
            report(false);
        });
        window.rutubeSwitchFailed = () => { changing = false; };
        window.rutubeSwitch = next => {
            if (!queue.some(item => item.id === next.id)) return;
            report(true);
            played = false;
            currentId = next.id;
            retries=0;
            retrying=false;
            season=info.find(item=>item.id===currentId)?.season || 0;
            art.controls['rutube-next'].style.display='none';
            changing = false;
            resumeAfterSwitch = true;
            art.currentTime = 0;
            restorePosition = Number(next.position) || 0;
            if (stream) stream.loadSource(next.url);
            else { art.url = next.url; resumeAfterSwitch = false; }
            updateEpisodes();
            updateSeason();
        };
        art.on('video:ended', () => {
            report(true);
            const index = queue.findIndex(item => item.id === currentId);
            if (autoNext && index >= 0 && index + 1 < queue.length && visible()) choose(queue[index+1].id);
        });
    }
    art.on('video:loadedmetadata', () => {
        if (restorePosition > 0) {
            art.currentTime = Math.min(restorePosition, Math.max(0, art.duration - 5));
            restorePosition = 0;
        }
    });
    if (!queue.length) art.on('video:timeupdate', () => report(false));
    window.rutubeFlush = () => report(true);
    art.on('video:pause', () => report(true));
    if (!queue.length) art.on('video:ended', () => report(true));
    art.on('video:playing', () => { played = true; art.controls['rutube-retry'].style.display='none'; report(true); send('Воспроизведение · Rutube'); });
    art.on('fullscreen', value => dioxus.send({fullscreen:!!value}));
    art.on('destroy', () => { clearTimeout(retryTimer); pending.clear(); window.rutubeRecover=null; window.rutubeRetryFailed=null; window.rutubeFlush=null; window.rutubeSwitch=null; window.rutubeSwitchFailed=null; dioxus.send({closed:true}); });
    art.on('video:error', () => send('Не удалось воспроизвести видео. Возможны ограничения CDN или неподдерживаемый кодек.'));
    window.rutubePlayer = art;
    (document.getElementById('rutube-watch-space') || document.getElementById('rutube-player')).scrollIntoView({block: 'start'});
};
