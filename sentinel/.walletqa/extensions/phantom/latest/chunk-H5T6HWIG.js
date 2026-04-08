import{a as I}from"./chunk-PPEL22JW.js";import{C as k,K as w,j as p,q as E}from"./chunk-C2WW32PJ.js";import{b as l}from"./chunk-KBBT3XPC.js";import{b as c,d as f}from"./chunk-2PW2PH4X.js";import{Na as C}from"./chunk-4J3SESBJ.js";import{a as i}from"./chunk-EJAQ3Z2J.js";import{a as O}from"./chunk-OJPBMZQC.js";import{W as x}from"./chunk-ULZ3YIG4.js";import{$b as S,Cc as R,Lh as _,jc as d,kc as u,nc as b}from"./chunk-JWTAN66J.js";import{Ca as m,I as g,J as v}from"./chunk-UIH6NVAU.js";import{a,i as o,j as r,n}from"./chunk-TSHWMJEM.js";o();n();var T=x({authRepository:l,queryClient:C});o();n();var ee=new _(i,l,T);o();n();var le=new S(I({isWriter:!1}),{fetch(e,t){return R.api().bearer(!0).fetch(e,t)}});o();n();var y,$=new O,J=a(()=>{if(r.ENVIRONMENT!=="e2e")return null;let e=globalThis.__PHANTOM_E2E_SEEDLESS_JUICEBOX_CLIENT__;return e||null},"getE2EJuiceboxClientOverride"),M=a(async()=>{let e=J();return e||y||(y=new E(new w),y)},"juiceboxClient"),A={storage:$,authRepository:l,juiceboxClient:M},D=a(()=>{if(r.ENVIRONMENT!=="e2e")return null;let e=globalThis.__PHANTOM_E2E_SEEDLESS_REPOSITORY_OVERRIDES__;return e||null},"getE2ESeedlessRepositoryOverrides"),q=k(A),h={...q,recover:a(async e=>{let t=D()?.recover;return t?await t(e):await q.recover(e)},"recover")};h.subscribe(p.RotationResult,({type:e,didRotate:t})=>{let s=`Se*dless Bundle Rotation Result: ${e}, didRotate: ${t}`;b.addBreadcrumb(u.Seedless,s,d.Info),i.capture("seedlessBundleRotationResult",{data:{type:e,didRotate:t}})});h.subscribe(p.RecoverResult,({type:e,reason:t})=>{let s=`Se*dless Bundle Recover Result: ${e}`;t&&(s+=`, reason: ${t}`),b.addBreadcrumb(u.Seedless,s,d.Info),i.capture("seedlessBundleRecoverResult",{data:{type:e,reason:t}})});h.subscribe(p.BackupResult,({type:e,didBackup:t})=>{let s=`Se*dless Bundle Backup Result: ${e}, didBackup: ${t}`;b.addBreadcrumb(u.Seedless,s,d.Info),i.capture("seedlessBundleBackupResult",{data:{type:e,didBackup:t}})});o();n();o();n();var L=function(e,t){return Object.defineProperty?Object.defineProperty(e,"raw",{value:t}):e.raw=t,e},N=c(B||(B=L([`
/* http://meyerweb.com/eric/tools/css/reset/
   v5.0.1 | 20191019
   License: none (public domain)
*/

html, body, div, span, applet, object, iframe,
h1, h2, h3, h4, h5, h6, p, blockquote, pre,
a, abbr, acronym, address, big, cite, code,
del, dfn, em, img, ins, kbd, q, s, samp,
small, strike, strong, sub, sup, tt, var,
b, u, i, center,
dl, dt, dd, menu, ol, ul, li,
fieldset, form, label, legend,
table, caption, tbody, tfoot, thead, tr, th, td,
article, aside, canvas, details, embed,
figure, figcaption, footer, header, hgroup,
main, menu, nav, output, ruby, section, summary,
time, mark, audio, video {
  margin: 0;
  padding: 0;
  border: 0;
  font-size: 100%;
  font: inherit;
  vertical-align: baseline;
}
/* HTML5 display-role reset for older browsers */
article, aside, details, figcaption, figure,
footer, header, hgroup, main, menu, nav, section {
  display: block;
}
/* HTML5 hidden-attribute fix for newer browsers */
*[hidden] {
    display: none;
}
body {
  line-height: 1;
}
menu, ol, ul {
  list-style: none;
}
blockquote, q {
  quotes: none;
}
blockquote:before, blockquote:after,
q:before, q:after {
  content: '';
  content: none;
}
table {
  border-collapse: collapse;
  border-spacing: 0;
}
`],[`
/* http://meyerweb.com/eric/tools/css/reset/
   v5.0.1 | 20191019
   License: none (public domain)
*/

html, body, div, span, applet, object, iframe,
h1, h2, h3, h4, h5, h6, p, blockquote, pre,
a, abbr, acronym, address, big, cite, code,
del, dfn, em, img, ins, kbd, q, s, samp,
small, strike, strong, sub, sup, tt, var,
b, u, i, center,
dl, dt, dd, menu, ol, ul, li,
fieldset, form, label, legend,
table, caption, tbody, tfoot, thead, tr, th, td,
article, aside, canvas, details, embed,
figure, figcaption, footer, header, hgroup,
main, menu, nav, output, ruby, section, summary,
time, mark, audio, video {
  margin: 0;
  padding: 0;
  border: 0;
  font-size: 100%;
  font: inherit;
  vertical-align: baseline;
}
/* HTML5 display-role reset for older browsers */
article, aside, details, figcaption, figure,
footer, header, hgroup, main, menu, nav, section {
  display: block;
}
/* HTML5 hidden-attribute fix for newer browsers */
*[hidden] {
    display: none;
}
body {
  line-height: 1;
}
menu, ol, ul {
  list-style: none;
}
blockquote, q {
  quotes: none;
}
blockquote:before, blockquote:after,
q:before, q:after {
  content: '';
  content: none;
}
table {
  border-collapse: collapse;
  border-spacing: 0;
}
`]))),Re=f(P||(P=L(["",""],["",""])),N),j=N,B,P;var H=c`
  ::-webkit-scrollbar {
    background: ${m.colors.legacy.areaBase};
    width: 7px;
  }

  ::-webkit-scrollbar-thumb {
    background: ${m.colors.legacy.elementBase};
    border-radius: 8px;
  }
`,z=c`
  ::-webkit-scrollbar {
    display: none;
  }
  * {
    scrollbar-width: none; /* Also needed to disable scrollbar Firefox */
  }
`,Ie=f`
    ${j}

    body, html, * {
        box-sizing: border-box;
        font-family: 'Inter', 'Roboto', Arial;
        user-select: none;
        color: currentColor;
        -moz-osx-font-smoothing: grayscale;
        text-rendering: optimizeSpeed;
        -webkit-font-smoothing: antialiased;
    }
    input, textarea {
        -webkit-user-select: text;
        -khtml-user-select: text;
        -moz-user-select: text;
        -ms-user-select: text;
        user-select: text;
    }
    body {
        color: ${m.colors.legacy.textBase};
        background: ${e=>e.backgroundColor};
        min-height: 100vh;
        margin: 0;
        display: flex;
        justify-content: center;
        align-items: center;
    }
    *:focus, *:focus-within {
        outline-color: transparent !important;
        outline-style: none !important;
        outline-width: 0px !important;
    }

    ${g||v?z:H}
`;export{T as a,ee as b,le as c,h as d,Ie as e};
//# sourceMappingURL=chunk-H5T6HWIG.js.map
