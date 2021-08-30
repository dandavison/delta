pub const DIFF: &[u8] = r#"
commit 7ec4627902020cccd7b3f4fbc63e1b0d6b9798cd
Author: Evan You <yyx990803@gmail.com>
Date:   Thu Feb 21 08:52:15 2019 -0500

    fix: ensure generated scoped slot code is compatible with 2.5
    
    fix #9545

diff --git a/src/compiler/codegen/index.js b/src/compiler/codegen/index.js
index a64c3421..d433f756 100644
--- a/src/compiler/codegen/index.js
+++ b/src/compiler/codegen/index.js
@@ -409,9 +409,9 @@ function genScopedSlots (
     .join(',')
 
   return `scopedSlots:_u([${generatedSlots}]${
-    needsForceUpdate ? `,true` : ``
+    needsForceUpdate ? `,null,true` : ``
   }${
-    !needsForceUpdate && needsKey ? `,false,${hash(generatedSlots)}` : ``
+    !needsForceUpdate && needsKey ? `,null,false,${hash(generatedSlots)}` : ``
   })`
 }
 
diff --git a/src/core/instance/render-helpers/resolve-scoped-slots.js b/src/core/instance/render-helpers/resolve-scoped-slots.js
index 6439324b..f11ca000 100644
--- a/src/core/instance/render-helpers/resolve-scoped-slots.js
+++ b/src/core/instance/render-helpers/resolve-scoped-slots.js
@@ -2,15 +2,16 @@
 
 export function resolveScopedSlots (
   fns: ScopedSlotsData, // see flow/vnode
-  hasDynamicKeys: boolean,
-  contentHashKey: number,
-  res?: Object
+  res?: Object,
+  // the following are added in 2.6
+  hasDynamicKeys?: boolean,
+  contentHashKey?: number
 ): { [key: string]: Function, $stable: boolean } {
   res = res || { $stable: !hasDynamicKeys }
   for (let i = 0; i < fns.length; i++) {
     const slot = fns[i]
     if (Array.isArray(slot)) {
-      resolveScopedSlots(slot, hasDynamicKeys, null, res)
+      resolveScopedSlots(slot, res, hasDynamicKeys)
     } else if (slot) {
       // marker for reverse proxying v-slot without scope on this.$slots
       if (slot.proxy) {
@@ -20,7 +21,7 @@ export function resolveScopedSlots (
     }
   }
   if (contentHashKey) {
-    res.$key = contentHashKey
+    (res: any).$key = contentHashKey
   }
   return res
 }
diff --git a/test/unit/modules/compiler/codegen.spec.js b/test/unit/modules/compiler/codegen.spec.js
index 98c202dd..e56b2576 100644
--- a/test/unit/modules/compiler/codegen.spec.js
+++ b/test/unit/modules/compiler/codegen.spec.js
@@ -232,25 +232,25 @@ describe('codegen', () => {
   it('generate dynamic scoped slot', () => {
     assertCodegen(
       '<foo><template :slot="foo" slot-scope="bar">{{ bar }}</template></foo>',
-      `with(this){return _c('foo',{scopedSlots:_u([{key:foo,fn:function(bar){return [_v(_s(bar))]}}],true)})}`
+      `with(this){return _c('foo',{scopedSlots:_u([{key:foo,fn:function(bar){return [_v(_s(bar))]}}],null,true)})}`
     )
   })
 
   it('generate scoped slot with multiline v-if', () => {
     assertCodegen(
       '<foo><template v-if="\nshow\n" slot-scope="bar">{{ bar }}</template></foo>',
-      `with(this){return _c('foo',{scopedSlots:_u([{key:"default",fn:function(bar){return (\nshow\n)?[_v(_s(bar))]:undefined}}],true)})}`
+      `with(this){return _c('foo',{scopedSlots:_u([{key:"default",fn:function(bar){return (\nshow\n)?[_v(_s(bar))]:undefined}}],null,true)})}`
     )
     assertCodegen(
       '<foo><div v-if="\nshow\n" slot="foo" slot-scope="bar">{{ bar }}</div></foo>',
-      `with(this){return _c(\'foo\',{scopedSlots:_u([{key:"foo",fn:function(bar){return (\nshow\n)?_c(\'div\',{},[_v(_s(bar))]):_e()}}],true)})}`
+      `with(this){return _c(\'foo\',{scopedSlots:_u([{key:"foo",fn:function(bar){return (\nshow\n)?_c(\'div\',{},[_v(_s(bar))]):_e()}}],null,true)})}`
     )
   })
 
   it('generate scoped slot with new slot syntax', () => {
     assertCodegen(
       '<foo><template v-if="show" #default="bar">{{ bar }}</template></foo>',
-      `with(this){return _c('foo',{scopedSlots:_u([(show)?{key:"default",fn:function(bar){return [_v(_s(bar))]}}:null],true)})}`
+      `with(this){return _c('foo',{scopedSlots:_u([(show)?{key:"default",fn:function(bar){return [_v(_s(bar))]}}:null],null,true)})}`
     )
   })
 "#.as_bytes();
