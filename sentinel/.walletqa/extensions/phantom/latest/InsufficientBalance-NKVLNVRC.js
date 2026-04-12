import{a as f,c as m}from"./chunk-IAQBUKHW.js";import{a as F}from"./chunk-2LX7OAGD.js";import"./chunk-GYP3SHVM.js";import{C as w,T as R}from"./chunk-U76AGJQH.js";import"./chunk-K7GUWPOD.js";import"./chunk-VIKLLUFD.js";import"./chunk-OJXKNWRD.js";import"./chunk-U3AK75BX.js";import"./chunk-K3AOJOH2.js";import"./chunk-IKHGLDQR.js";import"./chunk-Y7VKGRRM.js";import"./chunk-S6QG2THO.js";import"./chunk-DUNKZ5IF.js";import"./chunk-52WVDG57.js";import"./chunk-3L25NYXH.js";import"./chunk-6FWHOWU6.js";import"./chunk-YEWLBR7H.js";import"./chunk-6KBFFQTX.js";import"./chunk-M3PK3S7R.js";import"./chunk-IKD4USNT.js";import"./chunk-VRSJRQ5K.js";import"./chunk-H3R3NFRZ.js";import"./chunk-FKKJRVFA.js";import"./chunk-H4R4WCV5.js";import"./chunk-TEI22EPU.js";import"./chunk-POEQUK3L.js";import"./chunk-XNCXNXWV.js";import"./chunk-BRQH5KZA.js";import"./chunk-4FP76AVJ.js";import"./chunk-C2WW32PJ.js";import"./chunk-EOII3ZM4.js";import"./chunk-4AQPJCXC.js";import"./chunk-2YZIJOT3.js";import"./chunk-3V3CMLD7.js";import"./chunk-VLMAPQCU.js";import{c as T,d as b}from"./chunk-3MAR52KN.js";import{_a as s}from"./chunk-4GHA7GV2.js";import{c as t}from"./chunk-2PW2PH4X.js";import"./chunk-IBHGQ57S.js";import"./chunk-JQE54VLJ.js";import"./chunk-4J3SESBJ.js";import"./chunk-3ELOFJIA.js";import"./chunk-WXSO7J6E.js";import"./chunk-CI44BFID.js";import"./chunk-PAALQIFC.js";import"./chunk-HOXBCK7A.js";import"./chunk-K3BGCWMV.js";import"./chunk-EJAQ3Z2J.js";import"./chunk-OJPBMZQC.js";import"./chunk-M73UGOFM.js";import"./chunk-UPPQC44E.js";import"./chunk-CYENH7PC.js";import"./chunk-ULZ3YIG4.js";import"./chunk-5RA4IS22.js";import{Lb as B,ob as l,vb as x}from"./chunk-JWTAN66J.js";import"./chunk-AW2XPS6Y.js";import"./chunk-BYU664DD.js";import{Ca as a,M,Ma as I,N as h,Z as C}from"./chunk-UIH6NVAU.js";import"./chunk-U7OZEJ4F.js";import"./chunk-ZRGHR2IN.js";import{a as d,g as c,i as y,n as g}from"./chunk-TSHWMJEM.js";y();g();var k=c(M(),1);var n=c(h(),1),E=t.div`
  width: 100%;
  height: 100%;
  display: flex;
  flex-direction: column;
  justify-content: space-between;
  overflow-y: scroll;
`,N=t.div`
  display: flex;
  flex-direction: column;
  align-items: center;
  margin-top: 90px;
`,S=t(s).attrs({size:28,weight:500,color:a.colors.legacy.textBase})`
  margin: 16px;
`,V=t(s).attrs({size:14,weight:400,lineHeight:17,color:a.colors.legacy.textDiminished})`
  max-width: 275px;

  span {
    color: white;
  }
`,$=d(({networkId:o,token:r})=>{let{t:e}=C(),{handleHideModalVisibility:p}=R(),u=(0,k.useCallback)(()=>{p("insufficientBalance")},[p]),v=o&&x(B(l.getChainID(o))),{canBuy:P,openBuy:D}=w({caip19:v||"",context:"modal",analyticsEvent:"fiatOnrampFromInsufficientBalance",entryPoint:"insufficientBalance"}),i=o?l.getTokenSymbol(o):e("tokens");return(0,n.jsxs)(E,{children:[(0,n.jsx)("div",{children:(0,n.jsxs)(N,{children:[(0,n.jsx)(F,{type:"failure",backgroundWidth:75}),(0,n.jsx)(S,{children:e("insufficientBalancePrimaryText",{tokenSymbol:i})}),(0,n.jsx)(V,{children:e("insufficientBalanceSecondaryText",{tokenSymbol:i})}),r?(0,n.jsxs)(I,{borderRadius:8,gap:1,marginTop:32,width:"100%",children:[(0,n.jsx)(f,{label:e("insufficientBalanceRemaining"),children:(0,n.jsx)(m,{color:a.colors.legacy.spotNegative,children:`${r.balance} ${i}`})}),(0,n.jsx)(f,{label:e("insufficientBalanceRequired"),children:(0,n.jsx)(m,{children:`${r.required} ${i}`})})]}):null]})}),P?(0,n.jsx)(b,{primaryText:e("buyAssetInterpolated",{tokenSymbol:i}),onPrimaryClicked:D,secondaryText:e("commandCancel"),onSecondaryClicked:u}):(0,n.jsx)(T,{onClick:u,children:e("commandCancel")})]})},"InsufficientBalance"),X=$;export{$ as InsufficientBalance,X as default};
//# sourceMappingURL=InsufficientBalance-NKVLNVRC.js.map
