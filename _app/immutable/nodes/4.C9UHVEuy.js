import{s as F,e as b,a as I,t as j,c as g,b as P,w as D,g as S,d as E,f as $,m as A,i as N,h as _,j as H,k as U}from"../chunks/scheduler.BVXc--wO.js";import{S as G,i as K,a as v,g as L,t as w,c as Q,b as V,d as M,m as O,e as T}from"../chunks/index.CXEoh7lO.js";import{n as J,w as X}from"../chunks/index.DQoaeX7X.js";import{M as R}from"../chunks/MessageBox.Dn6QInaP.js";function z(s){let e,n;return e=new R({props:{type:"warning",$$slots:{default:[Z]},$$scope:{ctx:s}}}),{c(){V(e.$$.fragment)},l(t){M(e.$$.fragment,t)},m(t,l){O(e,t,l),n=!0},i(t){n||(v(e.$$.fragment,t),n=!0)},o(t){w(e.$$.fragment,t),n=!1},d(t){T(e,t)}}}function Z(s){let e;return{c(){e=j(`Your Near balance is low! Please top up your Near balance to not run out
      of gas.`)},l(n){e=E(n,`Your Near balance is low! Please top up your Near balance to not run out
      of gas.`)},m(n,t){N(n,e,t)},d(n){n&&$(e)}}}function ee(s){let e,n,t="Connected account:",l,a,o,p,f,d,C="Near balance:",c,u,h=(s[1]?s[1].format():"-")+"",x,q,B=s[1]!=null&&s[1].toNumber()<.5,y,r=B&&z(s);return{c(){e=b("div"),n=b("span"),n.textContent=t,l=I(),a=b("span"),o=j(s[0]),p=I(),f=b("div"),d=b("span"),d.textContent=C,c=I(),u=b("span"),x=j(h),q=I(),r&&r.c(),this.h()},l(i){e=g(i,"DIV",{class:!0});var m=P(e);n=g(m,"SPAN",{"data-svelte-h":!0}),D(n)!=="svelte-fsvay8"&&(n.textContent=t),l=S(m),a=g(m,"SPAN",{});var W=P(a);o=E(W,s[0]),W.forEach($),m.forEach($),p=S(i),f=g(i,"DIV",{class:!0});var k=P(f);d=g(k,"SPAN",{"data-svelte-h":!0}),D(d)!=="svelte-1syjy1u"&&(d.textContent=C),c=S(k),u=g(k,"SPAN",{});var Y=P(u);x=E(Y,h),Y.forEach($),q=S(k),r&&r.l(k),k.forEach($),this.h()},h(){A(e,"class","section-field"),A(f,"class","section-field")},m(i,m){N(i,e,m),_(e,n),_(e,l),_(e,a),_(a,o),N(i,p,m),N(i,f,m),_(f,d),_(f,c),_(f,u),_(u,x),_(f,q),r&&r.m(f,null),y=!0},p(i,[m]){(!y||m&1)&&H(o,i[0]),(!y||m&2)&&h!==(h=(i[1]?i[1].format():"-")+"")&&H(x,h),m&2&&(B=i[1]!=null&&i[1].toNumber()<.5),B?r?m&2&&v(r,1):(r=z(i),r.c(),v(r,1),r.m(f,null)):r&&(L(),w(r,1,1,()=>{r=null}),Q())},i(i){y||(v(r),y=!0)},o(i){w(r),y=!1},d(i){i&&($(e),$(p),$(f)),r&&r.d()}}}function te(s,e,n){let{accountId:t}=e,l;a();async function a(){const p=await(await fetch("https://near.lava.build/lava-referer-ccbfe99b-d205-4d9d-9b6e-e919d474e9c0",{method:"POST",headers:{"Content-Type":"application/json"},body:JSON.stringify({jsonrpc:"2.0",id:"dontcare",method:"query",params:{request_type:"view_account",finality:"final",account_id:t}})})).json();p.result&&n(1,l=new J(p.result.amount,24).sub(new J(p.result.locked,24)))}return s.$$set=o=>{"accountId"in o&&n(0,t=o.accountId)},[t,l]}class ne extends G{constructor(e){super(),K(this,e,te,ee,F,{accountId:0})}}function ae(s){let e,n;return e=new R({props:{type:"info",$$slots:{default:[le]},$$scope:{ctx:s}}}),{c(){V(e.$$.fragment)},l(t){M(e.$$.fragment,t)},m(t,l){O(e,t,l),n=!0},p(t,l){const a={};l&4&&(a.$$scope={dirty:l,ctx:t}),e.$set(a)},i(t){n||(v(e.$$.fragment,t),n=!0)},o(t){w(e.$$.fragment,t),n=!1},d(t){T(e,t)}}}function se(s){let e,n;return e=new ne({props:{accountId:s[0]}}),{c(){V(e.$$.fragment)},l(t){M(e.$$.fragment,t)},m(t,l){O(e,t,l),n=!0},p(t,l){const a={};l&1&&(a.accountId=t[0]),e.$set(a)},i(t){n||(v(e.$$.fragment,t),n=!0)},o(t){w(e.$$.fragment,t),n=!1},d(t){T(e,t)}}}function le(s){let e;return{c(){e=j("Please log in!")},l(n){e=E(n,"Please log in!")},m(n,t){N(n,e,t)},d(n){n&&$(e)}}}function oe(s){let e,n,t="Wallet",l,a,o,p;const f=[se,ae],d=[];function C(c,u){return c[0]?0:1}return a=C(s),o=d[a]=f[a](s),{c(){e=b("div"),n=b("h2"),n.textContent=t,l=I(),o.c(),this.h()},l(c){e=g(c,"DIV",{class:!0});var u=P(e);n=g(u,"H2",{class:!0,"data-svelte-h":!0}),D(n)!=="svelte-14vu7n3"&&(n.textContent=t),l=S(u),o.l(u),u.forEach($),this.h()},h(){A(n,"class","svelte-nil46c"),A(e,"class","page svelte-nil46c")},m(c,u){N(c,e,u),_(e,n),_(e,l),d[a].m(e,null),p=!0},p(c,[u]){let h=a;a=C(c),a===h?d[a].p(c,u):(L(),w(d[h],1,1,()=>{d[h]=null}),Q(),o=d[a],o?o.p(c,u):(o=d[a]=f[a](c),o.c()),v(o,1),o.m(e,null))},i(c){p||(v(o),p=!0)},o(c){w(o),p=!1},d(c){c&&$(e),d[a].d()}}}function re(s,e,n){let t;const l=X.accountId$;return U(s,l,a=>n(0,t=a)),[t,l]}class de extends G{constructor(e){super(),K(this,e,re,oe,F,{})}}export{de as component};
