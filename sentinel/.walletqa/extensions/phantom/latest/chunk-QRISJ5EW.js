import{a as x,b as K}from"./chunk-OMI34PMH.js";import{a as E}from"./chunk-5P5TJKKU.js";import{d as $}from"./chunk-6KBFFQTX.js";import{c as W}from"./chunk-M3PK3S7R.js";import{a as Q}from"./chunk-VRSJRQ5K.js";import{g as S,h as z}from"./chunk-H3R3NFRZ.js";import{c as P}from"./chunk-3MAR52KN.js";import{_a as g,u as I}from"./chunk-4GHA7GV2.js";import{c as i}from"./chunk-2PW2PH4X.js";import{b as H}from"./chunk-PAALQIFC.js";import{a as h}from"./chunk-K3BGCWMV.js";import{ae as b,ih as N,ob as f,ye as B}from"./chunk-JWTAN66J.js";import{Aa as v,Ca as r,M as w,N as l,Xa as D,Z as R}from"./chunk-UIH6NVAU.js";import{a as A,g as t,i as c,n as m}from"./chunk-TSHWMJEM.js";c();m();var y=t(w(),1);var C=t(l(),1),ae=y.default.memo(({chainAddress:s,onQRClick:n})=>{let{networkID:d,address:o}=s,{buttonText:a,copied:u,copy:T}=x(o),F=B(o,4),U=N(s.networkID),q=(0,y.useCallback)(G=>{G.stopPropagation(),T()},[T]);return(0,C.jsx)(D,{copied:u,copiedText:a,formattedAddress:F,networkBadge:(0,C.jsx)($,{networkID:d,address:o}),networkLogo:(0,C.jsx)(h,{networkID:d,size:40}),networkName:U,onCopyClick:q,onQRClick:n})});c();m();var O=t(K(),1),_=t(w(),1);c();m();var k=t(w(),1);var p=t(l(),1),Y=i.div`
  width: 100%;
`,Z=i(W)`
  background: ${r.colors.legacy.areaAccent};
  border: 1px solid ${r.colors.legacy.borderDiminished};
  border-radius: 6px 6px 0 0;
  border-bottom: none;
  margin: 0;
  padding: 16px 22px;
  font-size: 16px;
  font-weight: 500;
  line-height: 21px;
  text-align: center;
  resize: none;
  overflow: hidden;
`,j=i.button`
  display: flex;
  justify-content: center;
  align-items: center;
  background: ${r.colors.legacy.areaAccent};
  border: 1px solid ${r.colors.legacy.borderDiminished};
  border-radius: 0 0 6px 6px;
  border-top: none;
  height: 40px;
  width: 100%;
  padding: 0;
  cursor: pointer;

  &:hover {
    background: ${r.colors.brand.black};
  }
`,J=i(g).attrs({size:16,weight:600,lineHeight:16})`
  margin-left: 6px;
`,M=A(({value:s})=>{let{buttonText:n,copy:d}=x(s),o=(0,k.useRef)(null);return(0,k.useEffect)(()=>{A(()=>{if(o&&o.current){let u=o.current.scrollHeight;o.current.style.height=u+"px"}},"autoSizeTextArea")()},[]),(0,p.jsxs)(Y,{children:[(0,p.jsx)(Z,{ref:o,readOnly:!0,value:s}),(0,p.jsxs)(j,{onClick:d,children:[(0,p.jsx)(I,{}),(0,p.jsx)(J,{children:n})]})]})},"CopyArea");var e=t(l(),1),V=48,Qe=_.default.memo(({address:s,networkID:n,headerType:d,onCloseClick:o})=>{let{t:a}=R();return(0,e.jsxs)(e.Fragment,{children:[(0,e.jsx)(d==="page"?S:z,{children:a("depositAddress")}),(0,e.jsxs)(E,{children:[(0,e.jsx)(Q,{align:"center",justify:"center",id:"column",children:(0,e.jsxs)(ee,{id:"QRCodeWrapper",children:[(0,e.jsx)(X,{value:s,size:160,level:"Q",id:"styledqrcode"}),(0,e.jsx)(h,{networkID:n,size:V,borderColor:"areaBase",className:v({position:"absolute"})})]})}),(0,e.jsx)(g,{size:16,lineHeight:22,weight:600,margin:"16px 0 8px",children:a("depositAddressChainInterpolated",{chain:f.getNetworkName(n)})}),(0,e.jsx)(M,{value:s}),(0,e.jsxs)(g,{size:14,color:r.colors.legacy.textDiminished,lineHeight:20,margin:"16px 0",children:[(0,e.jsxs)(H,{i18nKey:"depositAssetQRCodeInterpolated",values:{network:f.getNetworkName(n)},children:["Use to receive tokens on the ",f.getNetworkName(n)," network only."]}),f.isBitcoinNetworkID(n)?(0,e.jsxs)(e.Fragment,{children:[" ",(0,e.jsx)(oe,{href:b,target:"_blank",rel:"noopener noreferrer",children:a("commandLearnMore2")})]}):null]})]}),(0,e.jsx)(P,{onClick:o,children:a("commandClose")})]})}),X=i(O.default)`
  padding: 8px;
  background: ${r.colors.legacy.white};
  border-radius: 6px;
  position: relative;
`,ee=i.div`
  display: flex;
  justify-content: center;
  align-items: center;
  width: 100%;
  height: 100%;
`,oe=i.a`
  color: ${r.colors.legacy.spotBase};
  text-decoration: none;
  font-weight: 500;
`;export{ae as a,M as b,Qe as c};
//# sourceMappingURL=chunk-QRISJ5EW.js.map
