function e(e,n,t,r){return new(t||(t=Promise))((function(o,i){function c(e){try{u(r.next(e))}catch(e){i(e)}}function a(e){try{u(r.throw(e))}catch(e){i(e)}}function u(e){var n;e.done?o(e.value):(n=e.value,n instanceof t?n:new t((function(e){e(n)}))).then(c,a)}u((r=r.apply(e,n||[])).next())}))}function n(e,n){var t,r,o,i,c={label:0,sent:function(){if(1&o[0])throw o[1];return o[1]},trys:[],ops:[]};return i={next:a(0),throw:a(1),return:a(2)},"function"==typeof Symbol&&(i[Symbol.iterator]=function(){return this}),i;function a(a){return function(u){return function(a){if(t)throw new TypeError("Generator is already executing.");for(;i&&(i=0,a[0]&&(c=0)),c;)try{if(t=1,r&&(o=2&a[0]?r.return:a[0]?r.throw||((o=r.return)&&o.call(r),0):r.next)&&!(o=o.call(r,a[1])).done)return o;switch(r=0,o&&(a=[2&a[0],o.value]),a[0]){case 0:case 1:o=a;break;case 4:return c.label++,{value:a[1],done:!1};case 5:c.label++,r=a[1],a=[0];continue;case 7:a=c.ops.pop(),c.trys.pop();continue;default:if(!(o=c.trys,(o=o.length>0&&o[o.length-1])||6!==a[0]&&2!==a[0])){c=0;continue}if(3===a[0]&&(!o||a[1]>o[0]&&a[1]<o[3])){c.label=a[1];break}if(6===a[0]&&c.label<o[1]){c.label=o[1],o=a;break}if(o&&c.label<o[2]){c.label=o[2],c.ops.push(a);break}o[2]&&c.ops.pop(),c.trys.pop();continue}a=n.call(e,c)}catch(e){a=[6,e],r=0}finally{t=o=0}if(5&a[0])throw a[1];return{value:a[0]?a[1]:void 0,done:!0}}([a,u])}}}var t=Object.defineProperty;function r(e,n=!1){let t=window.crypto.getRandomValues(new Uint32Array(1))[0],r=`_${t}`;return Object.defineProperty(window,r,{value:t=>(n&&Reflect.deleteProperty(window,r),null==e?void 0:e(t)),writable:!1,configurable:!0}),t}async function o(e,n={}){return new Promise(((t,o)=>{let i=r((e=>{t(e),Reflect.deleteProperty(window,`_${c}`)}),!0),c=r((e=>{o(e),Reflect.deleteProperty(window,`_${i}`)}),!0);window.__TAURI_IPC__({cmd:e,callback:i,error:c,...n})}))}function i(e,n="asset"){let t=encodeURIComponent(e);return navigator.userAgent.includes("Windows")?`https://${n}.localhost/${t}`:`${n}://localhost/${t}`}function c(){return e(this,void 0,void 0,(function(){var e;return n(this,(function(n){switch(n.label){case 0:return[4,o("plugin:copper|get_auth_info")];case 1:return[2,{deviceCode:(e=n.sent()).device_code,userCode:e.user_code,verificationUri:e.verification_uri,expiresIn:e.expires_in,interval:e.interval,message:e.message}]}}))}))}function a(t){return e(this,void 0,void 0,(function(){var e;return n(this,(function(n){switch(n.label){case 0:return[4,o("plugin:copper|get_ms_token",{authInfo:t})];case 1:return[2,{tokenType:(e=n.sent()).token_type,scope:e.scope,expiresIn:e.expires_in,accessToken:e.access_token,refreshToken:e.refresh_token}]}}))}))}function u(t){return e(this,void 0,void 0,(function(){var e;return n(this,(function(n){switch(n.label){case 0:return[4,o("plugin:copper|refresh_ms_token",{authData:t})];case 1:return[2,{tokenType:(e=n.sent()).token_type,scope:e.scope,expiresIn:e.expires_in,accessToken:e.access_token,refreshToken:e.refresh_token}]}}))}))}function s(t){return e(this,void 0,void 0,(function(){var e;return n(this,(function(n){switch(n.label){case 0:return[4,o("plugin:copper|get_auth_data",{auth_info:t})];case 1:return[2,{accessToken:(e=n.sent()).access_token,refreshToken:e.refresh_token,uuid:e.uuid,username:e.username,expiresAt:e.expires_at,xuid:e.xuid}]}}))}))}((e,n)=>{for(var r in n)t(e,r,{get:n[r],enumerable:!0})})({},{convertFileSrc:()=>i,invoke:()=>o,transformCallback:()=>r});export{s as getAuthData,c as getAuthenticationInfo,a as getMicrosoftToken,u as refreshAuthToken};
