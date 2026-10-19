// 진단 비컨 수신 — 폰 실측(GPU 렌더러·GL 에러·상태줄·UA·URL)을 로그로 남긴다.
// wrangler pages deployment tail 로 실시간 열람.
export async function onRequestPost(context) {
    let body = '';
    try {
        body = await context.request.text();
    } catch (e) {
        body = 'unreadable';
    }
    console.log('DIAG_BEACON', body.slice(0, 900));
    return new Response('ok');
}
