import{a as Z}from"./chunk-MLBLFFLX.js";import{a as U}from"./chunk-P4JY6GXW.js";import{a as G}from"./chunk-HMI4BFKI.js";import{b as F}from"./chunk-ZWFWTIGQ.js";import{b as $}from"./chunk-U6U3HASP.js";import{T as K}from"./chunk-U76AGJQH.js";import{k as B}from"./chunk-K7GUWPOD.js";import"./chunk-VIKLLUFD.js";import"./chunk-OJXKNWRD.js";import"./chunk-U3AK75BX.js";import"./chunk-K3AOJOH2.js";import"./chunk-IKHGLDQR.js";import"./chunk-Y7VKGRRM.js";import"./chunk-S6QG2THO.js";import"./chunk-DUNKZ5IF.js";import"./chunk-52WVDG57.js";import{a as R}from"./chunk-3L25NYXH.js";import"./chunk-6FWHOWU6.js";import"./chunk-YEWLBR7H.js";import"./chunk-6KBFFQTX.js";import{g as Q}from"./chunk-M3PK3S7R.js";import"./chunk-IKD4USNT.js";import{a as O}from"./chunk-VRSJRQ5K.js";import"./chunk-H3R3NFRZ.js";import"./chunk-FKKJRVFA.js";import{a as V}from"./chunk-H4R4WCV5.js";import"./chunk-TEI22EPU.js";import"./chunk-POEQUK3L.js";import"./chunk-XNCXNXWV.js";import"./chunk-BRQH5KZA.js";import"./chunk-4FP76AVJ.js";import"./chunk-C2WW32PJ.js";import"./chunk-EOII3ZM4.js";import"./chunk-4AQPJCXC.js";import"./chunk-2YZIJOT3.js";import"./chunk-3V3CMLD7.js";import"./chunk-VLMAPQCU.js";import{c as z}from"./chunk-3MAR52KN.js";import{_a as H,x as D}from"./chunk-4GHA7GV2.js";import{c as s}from"./chunk-2PW2PH4X.js";import"./chunk-IBHGQ57S.js";import"./chunk-JQE54VLJ.js";import{aa as E,ha as A,ka as _,la as N}from"./chunk-4J3SESBJ.js";import"./chunk-3ELOFJIA.js";import"./chunk-WXSO7J6E.js";import"./chunk-CI44BFID.js";import"./chunk-PAALQIFC.js";import"./chunk-HOXBCK7A.js";import"./chunk-K3BGCWMV.js";import"./chunk-EJAQ3Z2J.js";import"./chunk-OJPBMZQC.js";import"./chunk-M73UGOFM.js";import"./chunk-UPPQC44E.js";import"./chunk-CYENH7PC.js";import{D as v}from"./chunk-ULZ3YIG4.js";import"./chunk-5RA4IS22.js";import{yd as W}from"./chunk-JWTAN66J.js";import"./chunk-AW2XPS6Y.js";import"./chunk-BYU664DD.js";import{Aa as L,Ca as w,Cb as k,Ib as P,M,N as b,Z as h}from"./chunk-UIH6NVAU.js";import"./chunk-U7OZEJ4F.js";import"./chunk-ZRGHR2IN.js";import{a as I,g as p,i as f,n as C}from"./chunk-TSHWMJEM.js";f();C();var n=p(M(),1);f();C();var X=p(M(),1);var o=p(b(),1),j=L({marginLeft:4}),ee=s(R).attrs({align:"center",padding:"10px"})`
  background-color: ${w.colors.legacy.elementBase};
  border-radius: 6px;
  height: 74px;
  margin: 4px 0;
`,te=s.div`
  display: flex;
  align-items: center;
`,oe=s(O)`
  flex: 1;
  min-width: 0;
  text-align: left;
  align-items: normal;
`,ie=s(H).attrs({size:16,weight:600,lineHeight:19,noWrap:!0,maxWidth:"175px",textAlign:"left"})``,ne=s(H).attrs({color:w.colors.legacy.textDiminished,size:14,lineHeight:17,noWrap:!0})`
  text-align: left;
  margin-top: 5px;
`,le=s.div`
  width: 55px;
  min-width: 55px;
  max-width: 55px;
  height: 55px;
  min-height: 55px;
  max-height: 55px;
  aspect-ratio: 1;
  margin-right: 10px;
  position: relative;
  display: flex;
  justify-content: center;
  align-items: center;
`,q=X.default.memo(e=>{let{t:l}=h(),{collection:i,unknownItem:c,isHidden:a,isSpam:r,onToggleHidden:g}=e,{name:d,id:u}=i,m=_(i),y=N(i),S=A(m?.media,"image",!1,"small"),x=d||m?.name||c;return(0,o.jsxs)(ee,{children:[(0,o.jsx)(le,{children:r&&a?(0,o.jsx)(Z,{width:32}):S?(0,o.jsx)(F,{uri:S}):(0,o.jsx)($,{type:"image",width:42})}),(0,o.jsx)(R,{children:(0,o.jsxs)(oe,{children:[(0,o.jsxs)(te,{children:[(0,o.jsx)(ie,{children:x}),r?(0,o.jsx)(D,{className:j,fill:w.colors.legacy.spotWarning,height:16,width:16}):null]}),(0,o.jsx)(ne,{children:l("collectiblesSearchNrOfItems",{nrOfItems:y})})]})}),(0,o.jsx)(U,{id:u,label:`${d} visible`,checked:!a,onChange:T=>{g(T.target.checked?"show":"hide")}})]})});var t=p(b(),1),se=74,ae=10,re=se+ae,me=20,ce=s.div`
  width: 100%;
  height: 100%;
  display: flex;
  flex-direction: column;
`,de=s.div`
  position: relative;
  width: 100%;
`,pe=I(()=>{let{handleHideModalVisibility:e}=K(),{data:l,isPending:i}=v(),{viewState:c,viewStateLoading:a}=E({account:l}),r=(0,n.useCallback)(()=>e("collectiblesVisibility"),[e]),g=(0,n.useMemo)(()=>({...c,handleCloseModal:r}),[r,c]),d=(0,n.useMemo)(()=>i||a,[i,a]);return{data:g,loading:d}},"useProps"),ge=n.default.memo(e=>{let{t:l}=h(),i=(0,n.useRef)(null);return(0,n.useEffect)(()=>{setTimeout(()=>i.current?.focus(),200)},[]),(0,t.jsxs)(t.Fragment,{children:[(0,t.jsx)(de,{children:(0,t.jsx)(Q,{ref:i,tabIndex:0,placeholder:l("assetListSearch"),maxLength:W,onChange:e.handleSearch,value:e.searchQuery,name:"Search collectibles"})}),(0,t.jsx)(B,{children:(0,t.jsx)(k,{children:({height:c,width:a})=>(0,t.jsx)(P,{style:{padding:`${me}px 0`},scrollToIndex:e.searchQuery!==e.debouncedSearchQuery?0:void 0,height:c,width:a,rowCount:e.listItems.length,rowHeight:re,rowRenderer:r=>(0,t.jsx)(he,{...r,data:e.listItems,unknownItem:l("assetListUnknownToken"),getIsHidden:e.getIsHidden,getIsSpam:e.getIsSpam,getSpamStatus:e.getSpamStatus,onToggleHidden:e.onToggleHidden})})})})]})}),he=I(e=>{let{index:l,data:i,style:c,unknownItem:a,getIsHidden:r,getIsSpam:g,getSpamStatus:d,onToggleHidden:u}=e,m=i[l],y=r(m),S=g(m),x=d(m),T=(0,n.useCallback)(J=>u({item:m,status:J}),[u,m]);return(0,t.jsx)("div",{style:c,children:(0,t.jsx)(q,{collection:m,unknownItem:a,isHidden:y,isSpam:S,spamStatus:x,onToggleHidden:T})})},"ResultRowWrapper"),ue=I(()=>{let{data:e,loading:l}=pe(),{t:i}=h();return(0,t.jsxs)(ce,{children:[l?(0,t.jsx)(G,{}):(0,t.jsx)(ge,{...e}),(0,t.jsx)(V,{children:(0,t.jsx)(z,{onClick:e.handleCloseModal,children:i("commandClose")})})]})},"CollectiblesVisibilityPage"),Ue=ue;export{ue as CollectiblesVisibilityPage,Ue as default};
//# sourceMappingURL=CollectiblesVisibilityPage-GY2U7DJC.js.map
