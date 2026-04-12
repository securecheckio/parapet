import{a as Q,c as $,d as J}from"./chunk-VIKLLUFD.js";import{c as Y}from"./chunk-U3AK75BX.js";import{b as q}from"./chunk-S6QG2THO.js";import{g as x}from"./chunk-H3R3NFRZ.js";import{a as _,d as K}from"./chunk-3MAR52KN.js";import{_a as a}from"./chunk-4GHA7GV2.js";import{c as i}from"./chunk-2PW2PH4X.js";import{J as U,S as O,W,_ as G,ia as R,w as H}from"./chunk-3ELOFJIA.js";import{Ka as N,L as I,nb as z,w as F}from"./chunk-CI44BFID.js";import{b as D}from"./chunk-PAALQIFC.js";import{ua as M}from"./chunk-ULZ3YIG4.js";import{Ka as P,La as V,Lb as L,Na as Ct,r as b,u as k,ye as B}from"./chunk-JWTAN66J.js";import{Ca as n,L as w,M as ht,N as E,Z as S}from"./chunk-UIH6NVAU.js";import{a as s,g as f,i as v,n as A}from"./chunk-TSHWMJEM.js";v();A();var p=f(ht(),1);var t=f(E(),1),Tt=s(r=>{let{t:e}=S(),{voteAccountPubkey:m}=r,{showStakeAccountCreateAndDelegateStatusModal:j,closeAllModals:tt}=q(),et=s(()=>{r.onClose(),tt()},"onCloseTxStatusView"),{data:ot}=M(b.Solana),{data:at}=N(),nt=at?.totalQuantityString??"";R(ot,z.STAKE_FUNGIBLE);let{cluster:it,connection:y}=I(),l=O(),rt=L(k.Solana),{data:st}=F({query:{data:rt}}),lt=st?.usd,o=(0,p.useMemo)(()=>l.results?.find(yt=>yt.voteAccountPubkey===m),[l.results,m]),mt=o?.info?.name??o?.info?.keybaseUsername??B(m),ct=Y(y),[c,C]=(0,p.useState)(""),d=w(c),g=P(1+(G(y).data??0)),h=W({balance:nt,cluster:it,rentExemptionMinimum:g}),dt=s(()=>C(h.toString()),"onSetMax"),ut=d.isLessThan(g),pt=d.isGreaterThan(h),gt=d.isFinite(),u=c&&ut?e("validatorViewAmountSOLRequiredToStakeInterpolated",{amount:g}):c&&pt?e("validatorViewInsufficientBalance"):"",ft=ct.isPending,T=gt&&!u&&!ft,St=s(()=>{j({lamports:V(d).toNumber(),votePubkey:m,usdPerSol:lt,onClose:et,validatorName:mt})},"onSubmit"),xt=o?.totalApy?U(o.totalApy):null;return(0,t.jsx)(vt,{children:l.isPending?(0,t.jsx)(_,{}):l.isError||!o?(0,t.jsxs)(t.Fragment,{children:[(0,t.jsx)(x,{children:e("validatorViewPrimaryText")}),(0,t.jsx)(X,{children:(0,t.jsxs)(a,{size:16,color:n.colors.legacy.textDiminished,lineHeight:20,children:[e("validatorViewErrorFetching")," ",l.error?.message??""]})})]}):(0,t.jsxs)(t.Fragment,{children:[(0,t.jsx)(x,{children:e("validatorViewPrimaryText")}),(0,t.jsxs)(X,{children:[(0,t.jsx)(a,{size:16,color:n.colors.legacy.textDiminished,lineHeight:20,margin:"0 0 20px 0",children:(0,t.jsxs)(D,{i18nKey:"validatorViewDescriptionInterpolated",children:["Choose how much SOL you\u2019d like to ",(0,t.jsx)("br",{}),"stake with this validator. ",(0,t.jsx)(Z,{href:H,children:"Learn more"})]})}),(0,t.jsx)(Q,{value:c,symbol:"SOL",alignSymbol:"right",buttonText:e("maxInputMax"),width:47,warning:!!u,onSetTarget:dt,onUserInput:C}),(0,t.jsx)(bt,{children:(0,t.jsx)(a,{color:u?n.colors.legacy.spotNegative:"transparent",size:16,textAlign:"left",children:u})}),(0,t.jsx)(wt,{onEdit:r.onClose}),(0,t.jsx)($,{identifier:o.voteAccountPubkey,name:o.info?.name,keybaseUsername:o.info?.keybaseUsername,iconUrl:o.info?.iconUrl,website:o.info?.website,data:[{label:e("validatorCardEstimatedApy"),value:(0,t.jsxs)(a,{textAlign:"right",weight:500,size:14,noWrap:!0,children:[xt,"%"]})},{label:e("validatorCardCommission"),value:(0,t.jsxs)(a,{textAlign:"right",weight:500,size:14,noWrap:!0,children:[o.commission,"%"]})},{label:e("validatorCardTotalStake"),value:(0,t.jsx)(a,{textAlign:"right",weight:500,size:14,noWrap:!0,children:(0,t.jsx)(J,{children:o.activatedStake})})}]})]}),(0,t.jsx)(At,{children:(0,t.jsx)(K,{primaryText:e("validatorViewActionButtonStake"),secondaryText:e("commandClose"),onPrimaryClicked:St,onSecondaryClicked:r.onClose,primaryTheme:T?"primary":"default",primaryDisabled:!T})})]})})},"StakeAmountPage"),ee=Tt,vt=i.div`
  display: grid;
  grid-template-rows: 42px auto 47px;
  height: 100%;
`,X=i.div`
  display: flex;
  flex-direction: column;
  align-items: center;
`,Z=i.a.attrs({target:"_blank",rel:"noopener noreferrer"})`
  color: ${n.colors.legacy.spotBase};
  text-decoration: none;
  cursor: pointer;
`,At=i.section`
  display: flex;
  gap: 15px;
`,bt=i.div`
  width: 100%;
`,kt=i(a)`
  width: 100%;
  margin-top: 15px;
  > a {
    color: ${n.colors.legacy.spotBase};
    cursor: pointer;
  }
`,wt=s(r=>{let{t:e}=S();return(0,t.jsxs)(kt,{size:16,color:n.colors.legacy.textDiminished,lineHeight:20,textAlign:"left",children:[e("validatorViewValidator")," \u2022 ",(0,t.jsx)(Z,{onClick:r.onEdit,children:e("commandEdit")})]})},"ValidatorSectionLabel");export{Tt as a,ee as b};
//# sourceMappingURL=chunk-63G4PQMY.js.map
