import{a as ro,b as K}from"./chunk-RJQRT3PY.js";import{b as G}from"./chunk-KBBT3XPC.js";import{n as z}from"./chunk-2YZIJOT3.js";import"./chunk-3V3CMLD7.js";import{a as A}from"./chunk-VLMAPQCU.js";import"./chunk-3MAR52KN.js";import{_a as p,d as J,n as $}from"./chunk-4GHA7GV2.js";import{a as M,c}from"./chunk-2PW2PH4X.js";import"./chunk-TGAS7BAT.js";import{h as _}from"./chunk-Y7NYUHKS.js";import{a as x}from"./chunk-QQJPKFTO.js";import"./chunk-IBHGQ57S.js";import"./chunk-JQE54VLJ.js";import"./chunk-4J3SESBJ.js";import"./chunk-3ELOFJIA.js";import{b as R,f as F}from"./chunk-WXSO7J6E.js";import"./chunk-CI44BFID.js";import{b as I}from"./chunk-PAALQIFC.js";import"./chunk-HOXBCK7A.js";import"./chunk-K3BGCWMV.js";import"./chunk-EJAQ3Z2J.js";import{a as E}from"./chunk-OJPBMZQC.js";import{V as D}from"./chunk-M73UGOFM.js";import{a as N}from"./chunk-UPPQC44E.js";import"./chunk-CYENH7PC.js";import"./chunk-ULZ3YIG4.js";import{jb as to}from"./chunk-5RA4IS22.js";import{Mc as P,Vb as v,dd as l,lc as U}from"./chunk-JWTAN66J.js";import"./chunk-AW2XPS6Y.js";import"./chunk-BYU664DD.js";import{Ca as f,M as T,N as m,Z as C}from"./chunk-UIH6NVAU.js";import"./chunk-U7OZEJ4F.js";import"./chunk-ZRGHR2IN.js";import{a as e,g as i,i as a,n as s}from"./chunk-TSHWMJEM.js";a();s();var No=i(T(),1);var Y=i(ro(),1);a();s();var h=i(T(),1);a();s();var Q=i(to(),1);var o=i(m(),1),g=f.colors.legacy.spotNegative,io=c.div`
  position: fixed;
  top: 0;
  left: 0;
  height: 100%;
  width: 100%;
  background-color: ${f.colors.brand.white};
  padding: clamp(24px, 16vh, 256px) 24px;
  box-sizing: border-box;
`,eo=c.div`
  margin-bottom: 24px;
  padding-bottom: 8vh;
`,no=c.div`
  max-width: 100ch;
  margin: auto;

  * {
    text-align: left;
  }
`,H=c.a`
  text-decoration: underline;
  color: ${g};
`,u=new E,V=e(({origin:t,subdomain:n,source:w})=>{let{t:d}=C(),Z=t?F(t):"",k=w||Z,j=t??"",L=new URL(j),O=L.hostname,W=n==="true"?O:k,S=(0,Q.toUnicode)(W),oo=e(async()=>{if(n==="true"){let y=await u.get(l.UserWhitelistSubdomains),r=JSON.parse(`${y}`);r?r.push(O):r=[O],r=[...new Set(r)],u.set(l.UserWhitelistSubdomains,JSON.stringify(r))}else{let y=await u.get(l.UserWhitelistedOrigins),r=JSON.parse(`${y}`);r?r.push(k):r=[k],r=[...new Set(r)],u.set(l.UserWhitelistedOrigins,JSON.stringify(r))}["http:","https:"].includes(L.protocol)&&self.location.assign(t)},"handleClick");return(0,o.jsx)(io,{children:(0,o.jsxs)(no,{children:[(0,o.jsx)(eo,{children:(0,o.jsx)($,{width:128,fill:f.colors.brand.white})}),(0,o.jsx)(p,{size:30,color:g,weight:"600",children:d("blocklistOriginDomainIsBlocked",{domainName:S||d("blocklistOriginThisDomain")})}),(0,o.jsx)(p,{color:g,children:d("blocklistOriginSiteIsMalicious")}),(0,o.jsx)(p,{color:g,children:(0,o.jsxs)(I,{i18nKey:"blocklistOriginCommunityDatabaseInterpolated",children:["This site has been flagged as part of a",(0,o.jsx)(H,{href:R,rel:"noopener",target:"_blank",children:"community-maintained database"}),"of known phishing websites and scams. If you believe the site has been flagged in error,",(0,o.jsx)(H,{href:R,rel:"noopener",target:"_blank",children:"please file an issue"}),"."]})}),W?(0,o.jsx)(p,{color:g,onClick:oo,hoverUnderline:!0,children:d("blocklistOriginIgnoreWarning",{domainName:S})}):(0,o.jsx)(o.Fragment,{})]})})},"BlocklistOrigin");var b=i(m(),1),ao=e(()=>{let t;try{t=new URLSearchParams(self.location.search).get("origin")||"",new URL(t)}catch{t=""}return t},"getOriginParam"),so=e(()=>new URLSearchParams(self.location.search).get("subdomain")||"","getSubdomainParam"),mo=e(()=>new URLSearchParams(self.location.search).get("source")||"","getSourceParam"),X=e(()=>{let t=(0,h.useMemo)(()=>ao(),[]),n=(0,h.useMemo)(()=>so(),[]),w=(0,h.useMemo)(()=>mo(),[]);return(0,b.jsx)(J,{future:{v7_startTransition:!0},children:(0,b.jsx)(z,{children:(0,b.jsx)(V,{origin:t,subdomain:n,source:w})})})},"Blocklist");var B=i(m(),1);N();v([[P,x]]);U.init({provider:K});await D(x);await _("frontend",G);var lo=document.getElementById("root"),co=(0,Y.createRoot)(lo);co.render((0,B.jsx)(M,{theme:A,children:(0,B.jsx)(X,{})}));
//# sourceMappingURL=Phishing.js.map
