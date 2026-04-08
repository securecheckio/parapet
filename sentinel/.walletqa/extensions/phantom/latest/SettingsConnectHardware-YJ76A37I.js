import{a as N,c as F,d as G,g as I}from"./chunk-IU2M4VCO.js";import{a as x}from"./chunk-HNCEHMDA.js";import"./chunk-2LX7OAGD.js";import{a as D}from"./chunk-ATBGOQSL.js";import"./chunk-3GHR4PMQ.js";import"./chunk-ONUSXHBV.js";import"./chunk-FFIFHTYB.js";import"./chunk-SCHSE6KX.js";import"./chunk-U76AGJQH.js";import"./chunk-K7GUWPOD.js";import"./chunk-VIKLLUFD.js";import"./chunk-OJXKNWRD.js";import"./chunk-U3AK75BX.js";import"./chunk-K3AOJOH2.js";import"./chunk-IKHGLDQR.js";import"./chunk-Y7VKGRRM.js";import"./chunk-S6QG2THO.js";import"./chunk-DUNKZ5IF.js";import"./chunk-52WVDG57.js";import{a as L}from"./chunk-3L25NYXH.js";import"./chunk-6FWHOWU6.js";import"./chunk-YEWLBR7H.js";import"./chunk-6KBFFQTX.js";import"./chunk-M3PK3S7R.js";import"./chunk-IKD4USNT.js";import"./chunk-VRSJRQ5K.js";import"./chunk-H3R3NFRZ.js";import{a as C}from"./chunk-FKKJRVFA.js";import"./chunk-H4R4WCV5.js";import"./chunk-7JLJ6RGR.js";import"./chunk-TEI22EPU.js";import"./chunk-POEQUK3L.js";import"./chunk-XNCXNXWV.js";import"./chunk-BRQH5KZA.js";import"./chunk-4FP76AVJ.js";import"./chunk-GFRFUC32.js";import"./chunk-3VZMOAO6.js";import"./chunk-C2WW32PJ.js";import"./chunk-EOII3ZM4.js";import"./chunk-4AQPJCXC.js";import"./chunk-2YZIJOT3.js";import"./chunk-3V3CMLD7.js";import"./chunk-VLMAPQCU.js";import"./chunk-3MAR52KN.js";import{q as _}from"./chunk-4GHA7GV2.js";import{c as s}from"./chunk-2PW2PH4X.js";import{a as y}from"./chunk-QQJPKFTO.js";import"./chunk-IBHGQ57S.js";import"./chunk-JQE54VLJ.js";import"./chunk-4J3SESBJ.js";import"./chunk-3ELOFJIA.js";import"./chunk-WXSO7J6E.js";import"./chunk-CI44BFID.js";import"./chunk-PAALQIFC.js";import"./chunk-HOXBCK7A.js";import"./chunk-K3BGCWMV.js";import"./chunk-EJAQ3Z2J.js";import"./chunk-OJPBMZQC.js";import"./chunk-M73UGOFM.js";import"./chunk-UPPQC44E.js";import"./chunk-CYENH7PC.js";import{s as $,z as O}from"./chunk-ULZ3YIG4.js";import"./chunk-5RA4IS22.js";import{_e as E,pf as P}from"./chunk-JWTAN66J.js";import"./chunk-AW2XPS6Y.js";import"./chunk-BYU664DD.js";import{Ca as e,M as z,N as u,Ya as R,ab as T}from"./chunk-UIH6NVAU.js";import"./chunk-U7OZEJ4F.js";import"./chunk-ZRGHR2IN.js";import{a as g,g as l,i as n,n as i}from"./chunk-TSHWMJEM.js";n();i();var f=l(z(),1);n();i();n();i();var M=s(C)`
  cursor: pointer;
  width: 24px;
  height: 24px;
  transition: background-color 200ms ease;
  background-color: ${t=>t.$isExpanded?e.colors.legacy.black:e.colors.legacy.elementAccent} !important;
  :hover {
    background-color: ${e.colors.legacy.gray};
    svg {
      fill: white;
    }
  }
  svg {
    fill: ${t=>t.$isExpanded?"white":e.colors.legacy.textDiminished};
    transition: fill 200ms ease;
    position: relative;
    ${t=>t.top?`top: ${t.top}px;`:""}
    ${t=>t.right?`right: ${t.right}px;`:""}
  }
`;var o=l(u(),1),K=s(L).attrs({justify:"space-between"})`
  background-color: ${e.colors.legacy.areaBase};
  padding: 10px 16px;
  border-bottom: 1px solid ${e.colors.legacy.borderDiminished};
  height: 46px;
  opacity: ${t=>t.opacity??"1"};
`,Q=s.div`
  display: flex;
  margin-left: 10px;
  > * {
    margin-right: 10px;
  }
`,W=s.div`
  width: 24px;
  height: 24px;
`,X=g(({onBackClick:t,totalSteps:c,currentStepIndex:d,isHidden:m,showBackButtonOnFirstStep:r,showBackButton:S=!0})=>(0,o.jsxs)(K,{opacity:m?0:1,children:[S&&(r||d!==0)?(0,o.jsx)(M,{right:1,onClick:t,children:(0,o.jsx)(_,{})}):(0,o.jsx)(W,{}),(0,o.jsx)(Q,{children:E(c).map(p=>{let h=p<=d?e.colors.legacy.spotBase:e.colors.legacy.elementAccent;return(0,o.jsx)(C,{diameter:12,color:h},p)})}),(0,o.jsx)(W,{})]}),"StepHeader");n();i();var a=l(u(),1),Z=g(()=>{let{mutateAsync:t}=O(),{hardwareStepStack:c,pushStep:d,popStep:m,currentStep:r,setOnConnectHardwareAccounts:S,setOnConnectHardwareDone:b,setExistingAccounts:p}=N(),{data:h=[],isFetched:H,isError:v}=$(),w=P(c,(k,q)=>k?.length===q.length),J=c.length>(w??[]).length,B=w?.length===0,U={initial:{x:B?0:J?150:-150,opacity:B?1:0},animate:{x:0,opacity:1},exit:{opacity:0},transition:{duration:.2}},V=(0,f.useCallback)(()=>{r()?.props.preventBack||(r()?.props.onBackCallback&&r()?.props.onBackCallback?.(),m())},[r,m]);return D(()=>{S(async k=>{await t(k),await y.set(x,!await y.get(x))}),b(()=>self.close()),d((0,a.jsx)(I,{}))},c.length===0),(0,f.useEffect)(()=>{p({data:h,isFetched:H,isError:v})},[h,H,v,p]),(0,a.jsxs)(F,{children:[(0,a.jsx)(X,{totalSteps:3,onBackClick:V,showBackButton:!r()?.props.preventBack,currentStepIndex:c.length-1}),(0,a.jsx)(R,{mode:"wait",children:(0,a.jsx)(T.div,{style:{display:"flex",flexGrow:1},...U,children:(0,a.jsx)(G,{children:r()})},`${c.length}_${w?.length}`)})]})},"SettingsConnectHardware"),Tt=Z;export{Tt as default};
//# sourceMappingURL=SettingsConnectHardware-YJ76A37I.js.map
