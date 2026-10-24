# DooTube: ข้อเสนอด้านสถาปัตยกรรม

สถานะ: **ข้อเสนอเพื่อการตัดสินใจ ไม่ใช่ข้อยืนยันการทำงานหรือการรับรองจาก YouTube/Apple**
ปรับปรุง: 2026-10-23  
บันทึกประวัติ: ข้อเสนอฉบับนี้เขียนก่อนเริ่ม implementation; ผลงานปัจจุบันและหลักฐานการทดสอบแยกไว้ใน [Implementation](implementation.md) และ [Verification](verification.md) ไม่ควรตีความรายละเอียดข้อเสนอทั้งหมดว่าเป็นฟีเจอร์ที่ส่งมอบแล้ว
เป้าหมาย UX และขอบเขตผลิตภัณฑ์: [Design brief](design-brief.md)

**ลำดับผลิตภัณฑ์ที่ผู้สร้างยืนยัน:** UI เรียบแบบ Mac เก็บเครื่องมือขั้นสูงไว้ให้เปิดเมื่อจำเป็น; พิสูจน์ playback แล้วส่งมอบ Watch ที่ลื่นสำหรับใช้ประจำวัน ก่อนเน้น Music depth/customization ตาม [Design brief](design-brief.md) การสำรวจ Music/EQ ช่วงต้นให้จำกัดขอบเขตเพื่อหลีกเลี่ยงทางตันด้านสถาปัตยกรรม ไม่ต้องสำรวจให้ครบก่อนเริ่มพัฒนา Watch และไม่เปลี่ยน policy/permission/capability gates ด้านล่าง การยืนยันลำดับนี้ไม่ใช่การอนุมัติ API หรือทางเลือกสถาปัตยกรรมที่เสนอ

## ข้อเสนอโดยย่อ

สร้าง DooTube เป็น **แอป macOS แบบ native จริง โดยใช้ Rust + AppKit ผ่าน `objc2` / `objc2-app-kit`** สำหรับหน้าต่าง เมนู การนำทาง คีย์ลัด ค่ากำหนด และ UI ที่ออกแบบเอง ใช้ `WKWebView` เฉพาะเป็นพื้นผิวบริการ/เล่นสื่อ YouTube หรือ YouTube Music อย่างเป็นทางการ ไม่ใช่แทนทั้งแอปด้วยหน้าเว็บ ผู้ใช้ควรได้แอป native สำหรับ watch/listen และ workflow ที่ดีขึ้น แต่การควบคุม catalog ของ YouTube โดยตรงมีขอบเขตและความเสี่ยงตามผู้ให้บริการ

มีสองแนวทางซึ่งต้องไม่ปะปนกัน:

1. **Supported baseline:** UI ของ DooTube เป็น native; ค้นหา/ดูเนื้อหาและเล่น YouTube Music ผ่าน UI เว็บไซต์ทางการใน `WKWebView` หรือ official embed; ใช้ YouTube Data API เฉพาะงานที่ API ระบุไว้ชัดเจน แนวทางนี้ไม่ให้ native UI เข้าถึง Music catalog/history/lyrics อย่างครบถ้วน
2. **Unofficial parity path (ยังไม่อนุมัติ):** native Music browsing ใช้ adapter แยกสำหรับ endpoint ภายใน/ไม่เป็นทางการ โดยยังเล่นผ่าน official player/site เท่านั้น ทางเทคนิคอาจทำให้ native catalog/library ใกล้ความต้องการมากขึ้น แต่ Google ระบุในนโยบายว่า **ห้ามใช้ undocumented APIs หากไม่มี express permission** และห้าม reverse-engineer บริการดังกล่าว จึงมีความเสี่ยงด้านนโยบาย/การบังคับใช้และ endpoint เปลี่ยนแปลงสูง ต้องเป็นการตัดสินใจโดยเจ้าของผลิตภัณฑ์หลังขออนุญาตหรือรับทราบผลจากที่ปรึกษานโยบาย/กฎหมาย ไม่ใช่ข้อเสนอให้เปิดใช้หรือพึ่งพาโดยปริยาย

เป้าหมายไม่ใช่การรับรอง feature parity อัตโนมัติ ไม่ลดเป้าหมายให้เหลือ launcher และไม่อ้างว่า DooTube มีสิ่งที่ Kaset ไม่มีโดยไม่มีหลักฐาน การทำ EQ ให้ YouTube เป็นส่วนแยก: **Core Audio process taps ทำให้มีเส้นทางทางเทคนิคที่ควรทดลอง แต่ไม่ได้ทำให้การดัดแปลงเสียงผ่าน YouTube policy ได้โดยอัตโนมัติ**

## ทางเลือก UI และการแบ่งงาน

### Native Rust UI (แนะนำ)

- ใช้ AppKit เป็นแอปหลักจาก Rust ผ่าน `objc2-app-kit`; ใช้ bridge ภาษา Swift/Objective-C เฉพาะจุดที่จำเป็นจริง เช่น API/ownership integration ไม่ย้าย UI ทั้งแอปไป SwiftUI โดยไม่ตัดสินใจชัดเจน
- Rust ดูแล app state, navigation, preference, queue/workflow ที่แอปเป็นเจ้าของ, adapter interfaces, persistence และ feature capability state
- AppKit ดูแลหน้าต่าง เมนู settings, keyboard shortcuts, accessibility, drag/drop, lifecycle และ native chrome
- ฝัง `WKWebView` ในพื้นที่เนื้อหาที่มองเห็นได้เมื่อแสดง YouTube player/site; ไม่ใช้ DOM injection หรือ private WebKit hooks เป็น API หลัก
- `SwiftUI/AppKit shell + Rust core` เป็นทางเลือกสำรองแบบผสมภาษา ไม่ใช่ Rust-native UI ส่วน GPUI เป็นตัวเลือกทดลองที่ upstream ยังระบุว่า active development/pre-1.0; ยังไม่มีเหตุผลให้เลือกเป็นฐานก่อนมีการวัดผล

### Native watch/listen โดยไม่บิดเบือนข้อจำกัด

UI native สามารถจัด workflow, navigation รอบ provider, preferences, queue ของแอป, history/bookmarks ที่ผู้ใช้สร้างเอง, ค้นหาแหล่งที่มี documented API, และสลับเข้าสู่ official Music/video surface ได้ แต่ต้องแสดง provenance และชี้ชัดว่า metadata/transport ส่วนใดมาจากเว็บไซต์หรือ API

YouTube Data API v3 เปิด resource ทั่วไป เช่น videos, channels, playlists, playlist items, search results และ subscriptions; ไม่ใช่ streaming API และไม่ใช่ YouTube Music API สำหรับ album/artist semantics, personalized recommendation/listening history, lyrics หรือ Music playback ตามเอกสารที่ตรวจสอบ ณ วันที่ข้างต้น ข้อมูลเหล่านี้ใน native UI จึง **conditional**: official Music web surface เป็นทางรองรับ; native Music catalog/lyrics/history ต้องรอ API ที่เผยแพร่/การอนุญาต หรือเลือก unofficial adapter พร้อมยอมรับความเสี่ยงอย่างชัดแจ้ง

## Playback, visibility และ player constraints

- Supported baseline คือ official YouTube/YouTube Music ใน `WKWebView` หรือ official embedded player; ให้ YouTube ควบคุม playback, ads, login, quality/DRM restrictions, captions และ feature changes
- สำหรับ embedded player ต้องให้ `HTTP Referer` ระบุ app identity/bundle ID ตามข้อกำหนด ใช้ OS-provided `WKWebView` บน macOS, player viewport อย่างน้อย **200×200 px**, และหากมี controls ต้องมีพื้นที่ให้แสดงครบ; YouTube แนะนำขนาด 480×270 px สำหรับ 16:9
- ห้ามวาง overlay/frame/visual element บังส่วนใดของ embedded player หรือ controls; ห้ามแก้รูปลักษณ์/branding นอก API ที่ระบุ และต้องปฏิบัติตาม autoplay/visibility rules การเล่นอัตโนมัติต้องเริ่มเมื่อ player visible และมากกว่าครึ่งหนึ่งของพื้นที่มองเห็น
- **อย่าซ่อน/ย่อ/ย้ายออกนอกจอ player หรือคง player ไว้เป็นแหล่งเสียงลับ** Google policy guide ระบุห้ามแยก audio/video component, audio-only player และ background playback; ยกตัวอย่างการเล่นต่อเมื่อหน้าต่างปิดหรือ minimized ว่าไม่อนุญาตสำหรับ API client การปิด/ย่อหน้าต่างหรือเปลี่ยนไป UI ที่ซ่อน player จึงต้องไม่ถูกนำเสนอเป็น background audio mode โดยไม่ผ่าน policy review และ player ต้องไม่ถูกบัง แม้มี native compact/miniplayer chrome
- Mini player/PiP เป็น capability ที่ต้องตรวจเป็นรายกรณี ไม่ใช่รับรองจากการใช้ AppKit/AVKit; หากเป็น embedded player ต้องคง official player ที่มองเห็นได้ ขนาดเพียงพอ และ controls ใช้ได้ ปิดหรือ pause ตาม lifecycle ที่นโยบาย/API กำหนด ห้ามใช้ player ที่ซ่อนอยู่เพื่อให้ได้ audio-only playback
- เว็บไซต์เต็มรูปแบบกับ embedded IFrame API ไม่เหมือนกันด้าน lifecycle/controls/policy; อย่าอนุมานว่าเงื่อนไขทุกข้อใช้กับ website เช่นเดียวกันโดยไม่อ่าน policy ฉบับปัจจุบัน อย่างไรก็ดีอย่าออกแบบให้ย้าย webview ไปเบื้องหลังหรือเล่นต่อเมื่อปิด/minimized จนกว่าจะยืนยันว่าทางที่เลือกอนุญาต
- Google OAuth ที่แอปขอใช้ YouTube Data API แยกจาก session ใน website: desktop app เป็น public client ใช้ system browser + PKCE และ scope ต่ำสุดที่ต้องใช้ ห้ามฝัง client secret แบบ confidential ห้ามย้าย/อ่าน Google website cookie เพื่อทำ OAuth; Google ปฏิเสธ OAuth ใน embedded user-agent เช่น `WKWebView`

## Audio Lab / EQ: เส้นทางทดลองและข้อห้ามแยกกัน

### ความเป็นไปได้ทางเทคนิค

Apple Core Audio มี process tap สำหรับจับ outgoing audio ของ process หรือ group of processes และต่อ tap เป็น input ผ่าน HAL aggregate device; `CATapDescription` มีตัวเลือก mute source เพื่อส่ง output ของ process ไปยัง tap แทนลำโพงโดยตรง Apple sample ระบุ `NSAudioCaptureUsageDescription` และการขอ system-audio-recording permission เมื่อเริ่ม capture ครั้งแรก เอกสาร Rust `coreaudio` ระบุ process objects (macOS 14.2+) และ feature สำหรับ process taps/aggregate devices จึงมีเส้นทางที่ควรพิสูจน์:

`WKWebView/ผู้ช่วย WebKit ที่ส่งเสียง → Core Audio process tap → aggregate device → Rust DSP (EQ) → output device`

Tap ให้สัญญาณเสียง output ที่ถูก decode/render แล้ว ไม่ใช่ไฟล์หรือ bitstream ต้นฉบับ; อาจถูก decode, mix, volume-adjust หรือ resample มาแล้ว สามารถเป็นข้อมูลเพียงพอสำหรับเปลี่ยน tonal response แต่ Apple ไม่รับประกัน source fidelity, lossless/bit-perfect, format หรือ latency เฉพาะกรณีนี้ และไม่ได้รับประกัน protected/DRM output จะถูกจับได้ การจับเสียงอย่างเดียวไม่ควรนำไปสู่การอ้างว่าได้คุณภาพต้นฉบับ

- Kaset 0.14.2 shipped permission text explicitly says it taps its own audio output for EQ and does not record screen content. This is direct observation of installed bundle copy, not a runtime test or proof of the underlying implementation's performance/compliance. The matching [version-pinned upstream README](https://github.com/sozercan/kaset/blob/e8e2c1bf4fe339da6439456b92a8a1700f7439e6/README.md) and [v0.14.2 release](https://github.com/sozercan/kaset/releases/tag/v0.14.2) describe product features; those are upstream claims, not independently exercised functionality.

### ข้อจำกัดและ policy gate

- ยังไม่ทราบว่าเสียงจาก WKWebView ออกมาจาก app PID หรือ WebKit helper process ใดบ้าง; ไม่ทราบว่า process isolation จะแยก YouTube tab ปัจจุบันได้หรือ helper ถูกใช้ร่วม/รีสตาร์ตหรือไม่ การ tap process group อาจจับเสียงหลาย process รวมกัน จึงห้ามเรียกว่า per-tab EQ จนกว่าจะพิสูจน์
- Route ที่ mute source แล้วส่งเสียงผ่าน DSP/output ต้องป้องกันทั้งเสียง dry+wet ซ้อนกันและ silence หาก permission/device/process หาย ต้องจัดการ sample rate/channel/device switch, headroom/clipping, latency และ video/audio sync drift; ต้องมี opt-in consent, visible on/off state, bypass ไป playback ปกติ, cleanup และไม่ persist/record buffer
- ใน real-time audio callback: ห้าม allocate, ใช้ blocking lock, ทำ network/filesystem I/O หรือคำนวณที่ไม่จำกัดเวลา; ใช้ preallocated buffers และเตรียม coefficient/state updates จาก thread อื่นก่อนสลับอย่างปลอดภัย
- การอนุญาตของ macOS ให้ capture เป็น **permission สำหรับ OS capture เท่านั้น** ไม่ใช่ Google OAuth, license, สิทธิ์ใช้ content หรือการอนุมัติ YouTube policy
- YouTube developer-policy guide ระบุว่า API client ต้องไม่ให้ผู้ใช้แยก audio/video component หรือ modify audio/video portions ของวิดีโอ และ policy ห้าม background player/audio-only playback; ดังนั้น EQ ที่ทำให้เสียง YouTube ถูกดัดแปลงมี **นโยบายความเสี่ยงสูง/อาจไม่อนุญาต** แม้ผ่าน public API และไม่ได้บันทึกเสียง ห้ามจัด Audio Lab ให้เป็น feature ที่รับรองแล้วก่อนตรวจสอบการใช้จริงและขอ written approval หากจำเป็น
- ห้าม bypass DRM/capture denial, แยก track/stream, ใช้ audio-only/hidden player หรือบันทึกเนื้อหาเพื่อ EQ หาก capture ไม่ได้ตามปกติ ให้แสดงว่า EQ ไม่พร้อมและเล่นผ่าน official player ปกติ
- แยก policy gate ออกจาก engineering gate: prototype ที่ผ่าน latency/quality ไม่ได้แปลว่า YouTube อนุญาต การ ship ต้องมี policy/legal review และ approval ที่จำเป็น; หากไม่ผ่านให้ Audio Lab มีเฉพาะแหล่งที่ DooTube มีสิทธิ์/PCM path โดยตรง หรือปิด EQ สำหรับ YouTube

## Core layers และข้อมูลขั้นต่ำ

1. **AppKit UI (Rust):** native watch/listen shell, controls รอบ content area, settings, global keyboard commands, accessibility และ responsive layout
2. **Application core (Rust):** state machine, queue/workflow, user preferences, native integrations และ source/capability presentation
3. **Provider adapters (Rust contracts, implementation แยก):** `OfficialWebSurface`, `YouTubeDataApi`, optional `UnofficialMusicAdapter` (disabled/unapproved), `CoreAudioTap` (Audio Lab candidate). ห้ามรวม metadata API, playback surface, และ OS audio capture เป็น provider เดียว
4. **Platform bridges:** `WKWebView` / AppKit / Core Audio / MediaPlayer ผ่าน Rust bindings หรือ Swift shim แคบ ๆ; main-thread ownership และ callback/audio real-time path แยกกัน
5. **Persistence:** เก็บ setting และ user-created state เท่าที่จำเป็น; แยก `UserData` จาก YouTube metadata/API data; ทำ retention, refresh, deletion และ OAuth revoke ตามนโยบายก่อนเพิ่ม cache

ข้อมูล domain ขั้นต่ำและมี provenance:

- `SourceId`: source ที่ชัดเจน เช่น `youtube_video`, `youtube_music_site`, `youtube_data_api`, `unofficial_music`
- `ContentRef`: source + native identifier + link ที่เปิดบน official surface; ไม่มี stream URL ที่ derive จาก private endpoint
- `CapabilityStatus`: `supported | unsupported | account_required | degraded | unknown`, เหตุผล, source URL/policy, `checked_with_app_version` และเวลาที่ตรวจ
- `PlaybackState`: สถานะจากแหล่งที่เชื่อถือได้ ไม่สมมติว่าปุ่ม native เท่ากับสถานะ player
- `AudioLabState`: `off | awaiting_permission | capturing | unavailable | error`, process scope ที่อธิบายแก่ผู้ใช้, ไม่เก็บ captured PCM

ยังไม่เสนอ plugin platform หรือ remote executable updates; provider adapter ที่ไม่ได้อนุมัติควรถูก compile-time/config gated และปิดโดยค่าเริ่มต้น ไม่ใช่ดาวน์โหลดโค้ดขณะ runtime

## Compatibility และ release policy

- บันทึก compatibility matrix ต่อ app release: macOS/WebKit version, official surface/API, login, player control, visible-player lifecycle, Music features, Audio Lab policy/permission state และ known limitation; สถานะมาจากการทดสอบและเอกสาร ไม่เดาจาก API availability
- ติดตาม YouTube Developer Policy, Required Minimum Functionality, API revision history และ Apple API availability; อัปเดตแอปผ่าน release ปกติที่ review แล้ว พร้อมบอกเมื่อ feature degrade/unsupported และ link เปิด YouTube ได้
- แสดง capability ต่อ feature ไม่ใช้คำว่า “รองรับ YouTube” แบบเหมารวม และไม่รับประกันว่าแอปรุ่นใหม่เท่ากับ YouTube feature parity; ความสามารถของ YouTube เปลี่ยนได้โดยไม่เกี่ยวกับ DooTube release
- Unofficial adapter ถ้าผ่าน product/legal gate ต้องมี adapter version, source/status แยกจาก supported APIs, kill switch ใน release, degradation UI และไม่ใช้ข้อมูลที่ดึงมาเป็นข้ออ้างว่าเป็น API ทางการ

## Feasibility spike และเกณฑ์ตัดสิน

ค่าด้านล่างเป็น **งบเป้าหมายที่เสนอเพื่อวัด ไม่ใช่ benchmark ที่มีแล้ว**:

- วัด chrome p95 <100 ms, state reaction <50 ms, ไม่มี app-owned main-thread stall >100 ms; วัด launch-to-usable/click-to-first-frame (เสนอ p95 ≤3 s บนอุปกรณ์/network ที่ระบุ), CPU/memory, buffering/drop frame ใน scenario 30 นาที
- กลยุทธ์ที่เสนอสำหรับความลื่นไหล: AppKit reusable/virtualized lists, bounded thumbnail cache, async I/O และ decode นอก main thread, ยกเลิก search/navigation requests ที่ stale, และห้าม rebuild library ทั้งชุดต่อ frame; benchmark CPU/memory ของ app **รวม WebKit helper processes** เทียบ Kaset ภายใต้ Mac/เครือข่าย/สื่อ/สิทธิ์เดียวกัน ตัวเลขเหล่านี้เป็นแนวทางและแผนวัด ไม่ใช่ผลที่ทำแล้ว
- ตรวจ playback/login/captions/search/queue/controls, referral identity, minimum visible player size, player visibility เมื่อ resize/minimize/close, ปิดการเล่นที่ policy ไม่อนุญาต, media keys/Now Playing และ PiP แยกเป็น capability
- EQ spike: enumerate process objects ระหว่างเล่น, ตรวจว่า audio output มาจาก process ใดและแยก tab ได้หรือไม่; prototype tap→DSP→output ที่ได้รับ permission, ตรวจ one audible path, bypass, no silence/duplicate, formats/headroom, added latency/A-V sync 30 นาที, CPU, process/device change, permission denial/revoke; ไม่ capture protected media และไม่เก็บ sample content
- ตรวจ native Music feature matrix โดยแยก official API/web capabilities กับ undocumented path; ห้ามนับ internal endpoint ที่ยังไม่อนุมัติเป็น acceptance criterion ของ supported baseline
- ทำ policy review สำหรับ official embed, background/window lifecycle, player controls, content/audio modification, OAuth, data retention และ native Music adapter ก่อน release ไม่ใช่หลัง feature เสร็จ

ผลลัพธ์ที่ต้องตัดสินใจ: (A) native app + official website/player สำหรับ Music; (B) ขอ Google API/express approval เพื่อ native Music/Audio Lab; หรือ (C) ประเมิน unofficial Music adapter โดยเจ้าของผลิตภัณฑ์รับทราบความเสี่ยง/ข้อจำกัดและออกแบบทางถอย หากไม่มี API/approval และเจ้าของไม่รับความเสี่ยง คุณสมบัติ native Music ที่ต้องใช้ private API จะไม่อยู่ใน scope ที่รับรองได้ แต่ DooTube ยังเป็น native watch/listen app ไม่ใช่ launcher โดยอัตโนมัติ

## เอกสารและแหล่งอ้างอิง

- [Design brief](design-brief.md) — product intent, user needs และ UX acceptance
- Google: [YouTube API Services Developer Policies](https://developers.google.com/youtube/terms/developer-policies), [Complying with YouTube's Developer Policies](https://developers.google.com/youtube/terms/developer-policies-guide), [Required Minimum Functionality](https://developers.google.com/youtube/terms/required-minimum-functionality)
- Google: [YouTube Data API overview](https://developers.google.com/youtube/data/getting-started), [API reference](https://developers.google.com/youtube/v3/docs), [OAuth 2.0 for Desktop Apps](https://developers.google.com/identity/protocols/oauth2/native-app)
- Apple: [WKWebView](https://developer.apple.com/documentation/webkit/wkwebview), [AVPlayer](https://developer.apple.com/documentation/avfoundation/avplayer), [Capturing system audio with Core Audio taps](https://developer.apple.com/documentation/coreaudio/capturing-system-audio-with-core-audio-taps), [`CATapDescription`](https://developer.apple.com/documentation/coreaudio/catapdescription), [ScreenCaptureKit audio capture](https://developer.apple.com/documentation/screencapturekit/scstreamconfiguration/capturesaudio), [AudioUnit](https://developer.apple.com/documentation/audiotoolbox/audiounit), [MPNowPlayingInfoCenter](https://developer.apple.com/documentation/mediaplayer/mpnowplayinginfocenter), [MPRemoteCommandCenter](https://developer.apple.com/documentation/mediaplayer/mpremotecommandcenter)
- Rust: [`objc2-app-kit`](https://docs.rs/objc2-app-kit/latest/objc2_app_kit/), [`coreaudio`](https://docs.rs/coreaudio/latest/coreaudio/)
