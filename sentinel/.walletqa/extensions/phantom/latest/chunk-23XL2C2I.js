import{e as he}from"./chunk-5P5TJKKU.js";import{a as z,c as K,d as H,e as O,f as _}from"./chunk-3T52FGZD.js";import{a as A}from"./chunk-3L25NYXH.js";import{a as ye,b as X}from"./chunk-M3PK3S7R.js";import{a as ae}from"./chunk-C2WW32PJ.js";import{f as ue,g as fe}from"./chunk-2YZIJOT3.js";import{_a as x,p as E}from"./chunk-4GHA7GV2.js";import{c as s}from"./chunk-2PW2PH4X.js";import{a as M}from"./chunk-K3BGCWMV.js";import{a as me}from"./chunk-EJAQ3Z2J.js";import{g as pe}from"./chunk-M73UGOFM.js";import{O as de,g as L,j as ie,ka as $,m as se,v as ce,za as le}from"./chunk-ULZ3YIG4.js";import{Bg as ne,M as Y,Yc as ee,kc as Z,nc as j,ob as g,ve as te,wg as re,ye as oe}from"./chunk-JWTAN66J.js";import{Ca as W,G as Q,H as J,M as V,N as h,Va as b,Z as I,mb as T}from"./chunk-UIH6NVAU.js";import{a as c,g as l,i as p,n as m}from"./chunk-TSHWMJEM.js";p();m();var ve=l(V(),1);p();m();var we=l(V(),1);var n=l(h(),1),Ae=c(({onChange:e,value:t,networkID:o})=>{let d=L(),i=(0,we.useMemo)(()=>{if(!o)return[];let S=g.getAddressTypes(o);return d.filter(f=>S.includes(f))},[d,o]);if(!i||i.length===0)return null;let u=i.includes(t)?t:i[0];return(0,n.jsx)(Fe,{onChange:e,value:u,children:({isExpanded:S})=>(0,n.jsxs)(n.Fragment,{children:[(0,n.jsx)(De,{isActive:S,children:(0,n.jsx)(Se,{networkID:o,addressType:u,children:(0,n.jsx)(ge,{children:(0,n.jsx)(E,{fill:W.colors.legacy.textDiminished,width:10})})})}),(0,n.jsx)(Pe,{portal:!1,children:(0,n.jsx)(O,{maxHeight:"300px",children:i?.filter(f=>f!==u)?.map(f=>(0,n.jsx)(Ne,{value:f,children:(0,n.jsx)(Se,{networkID:o,addressType:f})},f))})})]})})},"SelectAddressTypeDropdown"),Se=c(({addressType:e,networkID:t,children:o})=>!t||!e?null:(0,n.jsxs)(A,{justify:"space-between",children:[(0,n.jsxs)(A,{children:[(0,n.jsx)(M,{networkID:t,size:32}),(0,n.jsx)(Be,{children:Y.getDisplayName(e)})]}),o]}),"SelectRow"),Fe=s(z)`
  width: 100%;
  position: relative;
`,ge=s.div`
  display: inline-flex;
  line-height: 0;
`,De=s(({isActive:e,...t})=>(0,n.jsx)(K,{...t}))`
  padding: 8px 16px 8px 12px;

  ${ge} {
    svg {
      transform: rotate(${e=>e.isActive?"-180deg":"0"});
      transition: transform 0.2s ease-in-out;
    }
  }
`,Pe=s(H)`
  z-index: 2;
  width: 100%;
`,Ne=s(_)`
  padding: 8px 16px 8px 12px;
  min-height: 50px;
`,Be=s(x).attrs({size:16,weight:400,lineHeight:19,margin:"0 0 0 8px"})``;p();m();var a=l(h(),1),Le=s(z)`
  width: 100%;
  position: relative;
`,xe=s.div`
  display: inline-flex;
  line-height: 0;
`,We=s(({isActive:e,...t})=>(0,a.jsx)(K,{...t}))`
  padding: 8px 16px 8px 12px;

  ${xe} {
    svg {
      transform: rotate(${e=>e.isActive?"-180deg":"0"});
      transition: transform 0.2s ease-in-out;
    }
  }
`,Me=s(H)`
  z-index: 2;
  width: 100%;
`,Ee=s(_)`
  padding: 8px 16px 8px 12px;
  min-height: 50px;
`,ze=s(x).attrs({size:16,weight:400,lineHeight:19,margin:"0 0 0 8px"})``,Ce=c(({onChange:e,value:t})=>{let o=ie();return(0,a.jsx)(Le,{onChange:e,value:t,children:({isExpanded:d})=>(0,a.jsxs)(a.Fragment,{children:[(0,a.jsx)(We,{isActive:d,children:(0,a.jsx)(Ie,{networkID:t,children:(0,a.jsx)(xe,{children:(0,a.jsx)(E,{fill:W.colors.legacy.textDiminished,width:10})})})}),(0,a.jsx)(Me,{portal:!1,children:(0,a.jsx)(O,{maxHeight:"300px",children:o.filter(i=>i!==t).map(i=>(0,a.jsx)(Ee,{value:i,children:(0,a.jsx)(Ie,{networkID:i})},i))})})]})})},"SelectChainDropdown"),Ie=c(({networkID:e,children:t})=>(0,a.jsxs)(A,{justify:"space-between",children:[(0,a.jsxs)(A,{children:[(0,a.jsx)(M,{networkID:e,size:32}),(0,a.jsx)(ze,{children:g.getNetworkName(e)})]}),t]}),"SelectRow");var r=l(h(),1),Wt=c(({onClick:e,disabled:t})=>{let{t:o}=I(),d=se();return(0,r.jsx)(b,{topLeft:{text:o("addAccountImportWalletPrimaryText")},bottomLeft:{text:o(d?"addAccountImportWalletSolanaSecondaryText":"addAccountImportWalletSecondaryText")},start:(0,r.jsx)(T,{backgroundColor:"borderBase",color:"textBase",icon:"Download",shape:"circle",size:32}),onClick:e,disabled:t})},"ImportPrivateKeyButton"),Mt=c(({control:e,getValues:t,register:o,setValue:d,trigger:i,errors:u,nameValidations:S,privateKey:f,privateKeyValidations:D,addressPreview:R})=>{let{t:C}=I(),P=le(y=>y.editableAccountMetadata),N=t("networkID"),w=g.getAddressTypes(N),k=L(),v=k.filter(y=>w.includes(y));return(0,r.jsxs)(he,{children:[(0,r.jsx)($,{name:"networkID",control:e,render:({field:{onChange:y,value:U}})=>k.length===1?(0,r.jsx)(r.Fragment,{}):(0,r.jsx)(Ce,{onChange:B=>{y(B);let Te=g.getAddressTypes(B),Re=k.filter(ke=>Te.includes(ke));d("addressType",Re[0]),f&&i("privateKey")},value:U})}),(0,r.jsx)($,{name:"addressType",control:e,render:({field:{onChange:y,value:U}})=>v.length===1?(0,r.jsx)(r.Fragment,{}):(0,r.jsx)(Ae,{onChange:B=>{y(B),f&&i("privateKey")},value:U,networkID:N})}),(0,r.jsx)(X.WithWarning,{placeholder:C("addAccountImportAccountName"),defaultValue:P?.name,warning:!!u.name,warningMessage:u.name?.message,autoComplete:"off",maxLength:pe,...o("name",S)}),(0,r.jsx)(Oe.WithWarning,{placeholder:C("addAccountImportAccountPrivateKey"),defaultValue:"",warning:!!u.privateKey,warningMessage:u.privateKey?.message,autoComplete:"off",...o("privateKey",D)}),R?(0,r.jsx)(Ke,{label:C("settingsWalletAddress"),pubkey:R}):null]})},"ImportPrivateKeyFormInputStack"),Ke=ve.default.memo(({label:e,pubkey:t})=>(0,r.jsxs)(A,{justify:"space-between",align:"center",margin:"-7px 0 0",children:[(0,r.jsx)(x,{size:16,weight:600,children:e}),(0,r.jsx)(x,{size:16,children:oe(t,4)})]})),He=s(X).attrs({as:"textarea"})`
  height: 120px;
  text-align: start;
  resize: none;
  -webkit-text-security: disc;
  font-size: 16px;
`,Oe=ye(He);p();m();var q=l(h(),1),Vt=c(({onClick:e,disabled:t})=>{let{t:o}=I(),d=Q||J;return(0,q.jsx)(b,{topLeft:{text:o("addAccountHardwareWalletPrimaryText")},bottomLeft:{text:o("addAccountHardwareWalletSecondaryText")},start:(0,q.jsx)(T,{backgroundColor:"borderBase",color:"textBase",icon:"WalletHardware",shape:"circle",size:32}),onClick:e,disabled:t||d})},"ConnectHardwareWalletButton");p();m();var G=l(h(),1),Yt=c(({onClick:e,disabled:t})=>{let{t:o}=I();return(0,G.jsx)(b,{topLeft:{text:o("addAccountImportSeedPhrasePrimaryText")},bottomLeft:{text:o("addAccountImportSeedPhraseSecondaryText")},start:(0,G.jsx)(T,{backgroundColor:"borderBase",color:"textBase",icon:"File",shape:"circle",size:32}),onClick:e,disabled:t})},"ImportSecretPhraseButton");p();m();var be=l(V(),1);var po=c((e,t)=>{let{refreshSyncedStorage:o}=t??{},d=ne(),{mutateAsync:i}=de(),{mutateAsync:u}=ce(),S=ee();return{handleImportSeed:(0,be.useCallback)(async({mnemonic:D,accountMetas:R,accountName:C,offsetIndex:P=0,seedlessOnboardingType:N})=>{let w=await(e==="seed"?ue(D,R,C):fe(D,R,C));if(w.length===0)throw new Error("Failed to set seed phrase");if(await u({accounts:w,offsetIndex:P,refreshSyncedStorage:o}),await i({identifier:w[0].identifier}),e==="seedless"&&N===ae.SeedlessBackup)try{let v=new Set(w.map(y=>y.seedIdentifier));await Promise.all([...v].map(y=>d.addAuthFactor({secretIdentifier:y})))}catch{j.captureError(new Error("Unable to add auth factor for se*dless!"),Z.Auth)}let k=w.flatMap(v=>te(v));me.capture("addSeedAccount",{data:{walletIndex:P+1,numAccounts:w.length,accountType:e,walletAddresses:k}}),re(S)},[u,o,i,e,S,d])}},"useImportSeed");export{po as a,Ce as b,Wt as c,Mt as d,Vt as e,Yt as f};
//# sourceMappingURL=chunk-23XL2C2I.js.map
