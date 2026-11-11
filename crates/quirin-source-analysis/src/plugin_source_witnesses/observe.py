# SPDX-License-Identifier: Apache-2.0
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#     http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

"""Private native observation producer. Output is untrusted until Rust replay.
No deployed-artifact claim: exact source projections, real headers, local DSOs,
and explicitly substituted allocation/list/log boundary helpers only.
"""
import hashlib,json,pathlib,re,shlex,subprocess,sys
inventory=json.loads(pathlib.Path(sys.argv[1]).read_text()); out=pathlib.Path(sys.argv[2]); out.mkdir(parents=True,exist_ok=True)
commands=[]
def run(args,cwd=None):
    p=subprocess.run(args,cwd=cwd,stdout=subprocess.PIPE,stderr=subprocess.PIPE)
    commands.append({'args':args,'cwd':str(cwd) if cwd else None,'exit':p.returncode,'stderr':p.stderr.decode(errors='replace')}); (out/'commands.json').write_text(json.dumps(commands,indent=2))
    if p.returncode: raise RuntimeError(json.dumps(commands[-1]))
    return p.stdout

def sha(b):return hashlib.sha256(b).hexdigest()
def flags(entry):
    a=shlex.split(entry['command']); result=[]; i=1
    while i<len(a):
        x=a[i];i+=1
        if x in ('-o','-c'): 
            if x=='-o':i+=1
            continue
        if x==entry['file'] or x=='-g':continue
        result.append(x)
    return result

def extraction(text,name):
    # Mask strings/comments without changing offsets; delimiters inside them
    # cannot terminate the exact original-byte function projection.
    mask=re.sub(r'/\*.*?\*/|//[^\n]*|"(?:\\.|[^"\\])*"|\'(?:\\.|[^\'\\])*\'',lambda m:''.join('\n' if c=='\n' else ' ' for c in m[0]),text,flags=re.S)
    matches=[m for m in re.finditer(r'(?m)^[^\n;{}]*\b'+re.escape(name)+r'\s*\([^;{}]*\)\s*\{',mask) if mask[m.start():mask.index(name,m.start())].strip() and not re.search(r'[=(]',mask[m.start():mask.index(name,m.start())])]
    if len(matches)!=1:raise ValueError(('ambiguous projection',name,len(matches)))
    m=matches[0];start=m.start();pos=mask.index('{',m.start());depth=1;end=pos+1
    while depth:
        c=mask[end];depth+=(c=='{')-(c=='}');end+=1
    return start,end,text[start:end]

dbbytes=pathlib.Path(inventory['actual_compile_database']).read_bytes()
if sha(dbbytes)!=inventory['compile_database_sha256']:raise ValueError('changed actual compile database')
db=json.loads(dbbytes)
for ce in inventory['source_compile_entries']:
    if ce['selected_entry'] not in db:raise ValueError('selected actual compile entry absent')
if run(['git','-C',inventory['pinned_source_root'],'rev-parse','HEAD']).decode().strip()!=inventory['source_revision']:raise ValueError('source revision mismatch')
sources=[]; projections=[]; asts={}; allflags=[]; header_inputs={}
function_sets=[['rd_kafka_plugin_cmp','rd_kafka_plugin_new','rd_kafka_plugin_destroy'],['is_input','is_filter','is_processor','is_output','get_handle','load_symbol','path_to_plugin_name','destroy_plugin','flb_plugin_load'],['flb_plugin_proxy_symbol','flb_plugin_proxy_register','flb_plugin_proxy_create','flb_plugin_proxy_destroy'],['proxy_go_output_register']]
for index,(sf,ce,names) in enumerate(zip(inventory['source_files'],inventory['source_compile_entries'],function_sets)):
    raw=pathlib.Path(sf['actual_path']).read_bytes()
    if sha(raw)!=sf['sha256']:raise ValueError('changed pinned source')
    text=raw.decode();sources.append(text); entry=ce['selected_entry']; f=flags(entry);allflags.append(f)
    # Original actual selected translation unit: full dependencies and PP bytes.
    dep=run(['/usr/bin/clang',*f,'-M',entry['file']],entry['directory']);(out/f'{index}.d').write_bytes(dep)
    dependencies=shlex.split(dep.decode().replace('\\\n',' ').split(':',1)[1])
    for spelling in dependencies:
        path=pathlib.Path(spelling); path=path if path.is_absolute() else pathlib.Path(entry['directory'])/path
        header_inputs[str(path)]=sha(path.read_bytes())
    pp=run(['/usr/bin/clang',*f,'-E',entry['file']],entry['directory']);(out/f'{index}.i').write_bytes(pp)
    for name in names:
        start,end,body=extraction(text,name)
        projections.append({'module':sf['path'],'function':name,'start':start,'end':end,'body':body,'sha256':sha(body.encode())})
        # Original, not instrumented projection, typed AST. Filter may emit
        # declaration and definition JSON objects, all retained for Rust checks.
        ast=run(['/usr/bin/clang',*f,'-fsyntax-only','-Xclang','-ast-dump=json','-Xclang','-ast-dump-filter','-Xclang',name,entry['file']],entry['directory'])
        asts[name]=ast.decode();(out/(name+'.ast.json')).write_bytes(ast)

def body(name):return next(x['body'] for x in projections if x['function']==name)
def prefix(index):
    text=sources[index]; # Original includes/defines only, no handwritten ABI.
    if index==0:return text[:text.index('/**')]
    first=['','static int is_input','static void proxy_cb_flush','int proxy_go_output_register'][index]
    return text[:text.index(first)]
common=r'''
#include <dlfcn.h>
#include <stddef.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
static const char *CASE; static int observing=1;
static void check_layout(const char *path,size_t expected,size_t alignment,size_t count,const size_t *offsets){
 void *h=dlopen(path,RTLD_LAZY);if(!h)return;size_t *actual=dlsym(h,"fixture_layout_size");size_t *a=dlsym(h,"fixture_layout_align");
 if(!actual||!a||*actual!=expected||*a!=alignment)exit(77);
 size_t *n=dlsym(h,"fixture_layout_count"),*v=dlsym(h,"fixture_layout_offsets");if(!n||!v||*n!=count)exit(80);for(size_t i=0;i<count;i++)if(v[i]!=offsets[i])exit(81);printf("LAYOUT:%zu:%zu",*actual,*a);for(size_t i=0;i<count;i++)printf(":%zu",v[i]);puts("");dlclose(h);
}
static void ev(const char *e){if(observing)printf("%s:%s\n",CASE,e);}
void fixture_event(const char *e){ev(e);}
static void *real_open(const char *p,int f){ev("open");return dlopen(p,f);}
static void *real_sym(void *h,const char *s){char b[160];snprintf(b,sizeof b,"symbol:%s",s);ev(b);return dlsym(h,s);}
static int real_close(void *h){ev("close");return dlclose(h);}
static void trace_free(void *p){ev("free");free(p);}
static void *trace_copy(void *d,const void *s,size_t n){void *r=memcpy(d,s,n);if(memcmp(d,s,n))exit(78);ev("copy");return r;}
static int alloc_count,fail_alloc;
static void *trace_malloc(size_t n){ev("allocate");if(++alloc_count==fail_alloc)return NULL;return malloc(n);}
static void *trace_calloc(size_t a,size_t b){ev("allocate");return calloc(a,b);}
static char *trace_strdup(const char *s){ev("allocate");return strdup(s);}
#define dlopen real_open
#define dlsym real_sym
#define dlclose real_close
#undef flb_error
#define flb_error(...) ((void)0)
#undef flb_errno
#define flb_errno() ((void)0)
#undef flb_malloc
#define flb_malloc trace_malloc
#undef flb_calloc
#define flb_calloc trace_calloc
#undef flb_strdup
#define flb_strdup trace_strdup
#undef flb_free
#define flb_free trace_free
#define memcpy trace_copy
'''
# True MK list primitives are retained behind an instrumentation boundary.
list_support=r'''
static void add_observed(struct mk_list *a,struct mk_list *b){mk_list_add(a,b);if(a->next->prev!=a||a->prev->next!=a)exit(79);ev("link");}
#define mk_list_add add_observed
static flb_sds_t sds_observed(const char *s){ev("retain");return strdup(s);}
#define flb_sds_create sds_observed
#define flb_sds_destroy trace_free
'''
def build(name,text,index,shared=False):
    p=out/(name+'.c');p.write_text('#include <stddef.h>\n'+text);target=out/(name+'.so' if shared else name)
    args=['/usr/bin/clang',*allflags[index],'-O0','-g0','-ffunction-sections','-fdata-sections','-Wl,--gc-sections','-Wl,--build-id=none','-rdynamic','-I'+str(pathlib.Path(inventory['source_files'][index]['actual_path']).parent),'-o',str(target),str(p),'-ldl']
    if shared:args.insert(1,'-shared')
    run(args);return target
# RK local DSOs export the real header's exact conf_init signature.
rkheader=prefix(0)
for name,code in [('rk_ok','rd_kafka_resp_err_t conf_init(rd_kafka_conf_t *c,void **o,char *e,size_t n){*o=(void*)0x123;return 0;}'),('rk_error','rd_kafka_resp_err_t conf_init(rd_kafka_conf_t *c,void **o,char *e,size_t n){return RD_KAFKA_RESP_ERR__INVALID_ARG;}'),('rk_empty','int empty_fixture;')]:build(name,rkheader+'\nsize_t fixture_layout_size=sizeof(rd_kafka_conf_t); size_t fixture_layout_align=_Alignof(rd_kafka_conf_t);size_t fixture_layout_count=2;size_t fixture_layout_offsets[]={offsetof(rd_kafka_conf_t,plugins),offsetof(rd_kafka_plugin_t,rkplug_handle)};\nextern void fixture_event(const char*);\n'+code.replace('{*o=', '{fixture_event("conf_init");*o=').replace('{return RD_KAFKA', '{fixture_event("conf_init");return RD_KAFKA'),0,True)
rkstubs=r'''
static int duplicate;static void *retained;
static void *find_stub(const rd_list_t *l,const void *p,int(*cmp)(const void*,const void*)){ev("duplicate_test");return duplicate?(void*)p:NULL;}
static void add_stub(rd_list_t *l,void *p){ev("retain");retained=p;}
static void *open_stub(const char *p,char *e,size_t n){return real_open(p,RTLD_LAZY);}
static void *sym_stub(void *h,const char *s,char *e,size_t n){return real_sym(h,s);}
#undef rd_kafka_dbg0
#define rd_kafka_dbg0(...) ((void)0)
#define rd_list_find find_stub
#define rd_list_add add_stub
#define rd_dl_open open_stub
#define rd_dl_sym sym_stub
#define rd_dl_close real_close
#undef rd_calloc
#define rd_calloc trace_calloc
#undef rd_strdup
#define rd_strdup trace_strdup
#undef rd_free
#define rd_free trace_free
'''
rkmain=r'''
int main(int argc,char **argv){rd_kafka_conf_t conf={0};char err[256];int r;CASE=argv[1];check_layout(argv[2],sizeof(rd_kafka_conf_t),_Alignof(rd_kafka_conf_t),2,(size_t[]){offsetof(rd_kafka_conf_t,plugins),offsetof(rd_kafka_plugin_t,rkplug_handle)});duplicate=!strcmp(CASE,"rk_duplicate");ev("begin");r=rd_kafka_plugin_new(&conf,argv[2],err,sizeof err);if(!strcmp(CASE,"rk_load_destroy")){if(r||!retained)return 2;rd_kafka_plugin_destroy(retained);}if(r==0)ev("return:0");else ev("return:error");ev("end");return 0;}
'''
rk=build('rk_projection',rkheader+common+rkstubs+'\n'.join(body(x) for x in function_sets[0])+rkmain,0)
# Real C registration layouts, no callback invoked by this fixture.
fbheader=prefix(1)
structs={'input':'flb_input_plugin','processor':'flb_processor_plugin','filter':'flb_filter_plugin','output':'flb_output_plugin'}
for kind,st in structs.items():
    nm={'input':'in_probe','processor':'processor_probe','filter':'filter_probe','output':'out_probe'}[kind]
    build('flb-'+nm,fbheader+f'\nstruct {st} {nm}_plugin={{0}}; size_t fixture_layout_size=sizeof(struct {st});size_t fixture_layout_align=_Alignof(struct {st});size_t fixture_layout_count=1;size_t fixture_layout_offsets[]={{offsetof(struct {st},_head)}};\n',1,True)
build('flb-in_missing',fbheader+'\nint missing_structure;size_t fixture_layout_size=sizeof(struct flb_input_plugin);size_t fixture_layout_align=_Alignof(struct flb_input_plugin);size_t fixture_layout_count=1;size_t fixture_layout_offsets[]={offsetof(struct flb_input_plugin,_head)};\n',1,True)
build('invalid',fbheader+'\nint invalid_name;size_t fixture_layout_size=sizeof(struct flb_input_plugin);size_t fixture_layout_align=_Alignof(struct flb_input_plugin);size_t fixture_layout_count=1;size_t fixture_layout_offsets[]={offsetof(struct flb_input_plugin,_head)};\n',1,True)
fbmain=r'''
int main(int argc,char **argv){struct flb_config config={0};struct flb_plugins ctx={0};int r;CASE=argv[1];size_t sz=sizeof(struct flb_input_plugin),al=_Alignof(struct flb_input_plugin);if(strstr(CASE,"processor_success")){sz=sizeof(struct flb_processor_plugin);al=_Alignof(struct flb_processor_plugin);}if(strstr(CASE,"filter_success")){sz=sizeof(struct flb_filter_plugin);al=_Alignof(struct flb_filter_plugin);}if(strstr(CASE,"output_success")){sz=sizeof(struct flb_output_plugin);al=_Alignof(struct flb_output_plugin);}size_t off=offsetof(struct flb_input_plugin,_head);if(strstr(CASE,"processor_success"))off=offsetof(struct flb_processor_plugin,_head);if(strstr(CASE,"filter_success"))off=offsetof(struct flb_filter_plugin,_head);if(strstr(CASE,"output_success"))off=offsetof(struct flb_output_plugin,_head);check_layout(argv[2],sz,al,1,&off);mk_list_init(&config.in_plugins);mk_list_init(&config.processor_plugins);mk_list_init(&config.filter_plugins);mk_list_init(&config.out_plugins);mk_list_init(&ctx.input);mk_list_init(&ctx.processor);mk_list_init(&ctx.filter);mk_list_init(&ctx.output);fail_alloc=!strcmp(CASE,"flb_c_context_alloc_fail")?3:0;ev("begin");r=flb_plugin_load(argv[2],&ctx,&config);ev(r==0?"return:0":"return:error");ev("end");return 0;}
'''
fb=build('fb_projection',fbheader+common+list_support+'\n'.join(body(x) for x in function_sets[1])+fbmain,1)
# Real Go/proxy headers and actual selected body; unselected linking helpers
# are declared explicit stubs, never counted as registration proof.
goheader=prefix(3);proxyheader=prefix(2)
# Relative ./go.h and proxy/go/go.h resolve against original source folders.
proxyheader=proxyheader.replace('"proxy/go/go.h"','"'+str(pathlib.Path(inventory['source_files'][3]['actual_path']).parent/'go.h')+'"')
goheader=goheader.replace('"./go.h"','"'+str(pathlib.Path(inventory['source_files'][3]['actual_path']).parent/'go.h')+'"')
golayout='\nsize_t fixture_layout_size=sizeof(struct flbgo_output_plugin);size_t fixture_layout_align=_Alignof(struct flbgo_output_plugin);size_t fixture_layout_count=3;size_t fixture_layout_offsets[]={offsetof(struct flbgo_output_plugin,cb_init),offsetof(struct flbgo_output_plugin,cb_exit_ctx),offsetof(struct flbgo_output_plugin,name)};\n'
go_base=r'''
int FLBPluginInit(struct flbgo_output_plugin *ctx){return 0;}
_Static_assert(__builtin_types_compatible_p(__typeof__(&FLBPluginInit),__typeof__(((struct flbgo_output_plugin*)0)->cb_init)),"actual Go init pointer ABI");
extern void fixture_event(const char*);
int FLBPluginRegister(struct flb_plugin_proxy_def *d){fixture_event("register");d->proxy=FLB_PROXY_GOLANG;d->type=FLB_PROXY_OUTPUT_PLUGIN;d->name="fixture";fixture_event("register:success");return 0;}
'''
build('go_ok',goheader+golayout+go_base,3,True)
build('go_pre_fail',goheader+golayout+go_base+'\nint FLBPluginPreRegister(int h){fixture_event("pre_register");return -1;}\n',3,True)
build('go_no_init',goheader+golayout+'\nint FLBPluginRegister(struct flb_plugin_proxy_def *d){return 0;}\n',3,True)
build('go_empty',goheader+golayout+'\nint no_register;\n',3,True)
proxystubs=r'''
static struct flb_api fixture_api;
static struct flb_api *api_stub(void){ev("allocate");return &fixture_api;}
static void api_destroy_stub(struct flb_api *a){}
#define flb_api_create api_stub
#define flb_api_destroy api_destroy_stub
static int flb_proxy_register_output(struct flb_plugin_proxy *p,struct flb_plugin_proxy_def *d,struct flb_config *c){ev("link");return 0;}
static int flb_proxy_register_input(struct flb_plugin_proxy *p,struct flb_plugin_proxy_def *d,struct flb_config *c){ev("link");return 0;}
static int flb_proxy_register_custom(struct flb_plugin_proxy *p,struct flb_plugin_proxy_def *d,struct flb_config *c){ev("link");return 0;}
int proxy_go_input_register(struct flb_plugin_proxy *p,struct flb_plugin_proxy_def *d){return -1;}
int proxy_go_custom_register(struct flb_plugin_proxy *p,struct flb_plugin_proxy_def *d){return -1;}
'''
gomain=r'''
int main(int argc,char **argv){struct flb_config c={0};struct flb_plugin_proxy p={0};struct flb_plugin_proxy_def d={0};int r;CASE=argv[1];check_layout(argv[2],sizeof(struct flbgo_output_plugin),_Alignof(struct flbgo_output_plugin),3,(size_t[]){offsetof(struct flbgo_output_plugin,cb_init),offsetof(struct flbgo_output_plugin,cb_exit_ctx),offsetof(struct flbgo_output_plugin,name)});mk_list_init(&c.proxies);ev("begin");if(!strcmp(CASE,"go_create_ignores_register_failure")){struct flb_plugin_proxy *created=flb_plugin_proxy_create(argv[2],FLB_PROXY_OUTPUT_PLUGIN,&c);if(!created)return 2;ev("return:nonnull");flb_plugin_proxy_destroy(created);}else{p.dso_handler=dlopen(argv[2],RTLD_LAZY);if(!p.dso_handler)return 3;p.def=&d;d.name="fixture";d.type=FLB_PROXY_OUTPUT_PLUGIN;if(!strcmp(CASE,"go_init_missing")||!strcmp(CASE,"go_optional_hooks_absent"))r=proxy_go_output_register(&p,&d);else r=flb_plugin_proxy_register(&p,&c);ev(r==0?"return:0":"return:error");if(p.data){struct flbgo_output_plugin *g=p.data;ev(g->cb_init?"init:present":"init:absent");if(!g->cb_flush&&!g->cb_flush_ctx&&!g->cb_exit&&!g->cb_exit_ctx)ev("optional:absent");}dlclose(p.dso_handler);}ev("end");return 0;}
'''
go=build('go_projection',proxyheader+goheader+common+list_support+proxystubs+body('flb_plugin_proxy_symbol')+body('proxy_go_output_register')+body('flb_plugin_proxy_register')+body('flb_plugin_proxy_create')+body('flb_plugin_proxy_destroy')+gomain,2)
paths={'rk_duplicate':'rk_ok.so','rk_open_fail':'absent.so','rk_conf_init_missing':'rk_empty.so','rk_conf_init_error':'rk_error.so','rk_load_success':'rk_ok.so','rk_load_destroy':'rk_ok.so','flb_c_open_fail':'absent.so','flb_c_invalid_name':'invalid.so','flb_c_missing_structure':'flb-in_missing.so','flb_c_input_success':'flb-in_probe.so','flb_c_processor_success':'flb-processor_probe.so','flb_c_filter_success':'flb-filter_probe.so','flb_c_output_success':'flb-out_probe.so','flb_c_context_alloc_fail':'flb-in_probe.so','go_pre_optional_absent':'go_ok.so','go_pre_failed':'go_pre_fail.so','go_register_missing':'go_empty.so','go_init_missing':'go_no_init.so','go_optional_hooks_absent':'go_ok.so','go_create_ignores_register_failure':'go_empty.so'}
traces={};layouts={}
for case in inventory['selected_case_domain']:
    binary=rk if case.startswith('rk_') else fb if case.startswith('flb_') else go
    lines=run([str(binary),case,str(out/paths[case])]).decode().splitlines()
    layout=[x for x in lines if x.startswith('LAYOUT:')];layouts[case]=layout
    lines=[x for x in lines if not x.startswith('LAYOUT:')]
    if any(not x.startswith(case+':') for x in lines):raise ValueError('trace framing')
    traces[case]=[x[len(case)+1:] for x in lines]
artifacts={p.name:sha(p.read_bytes()) for p in sorted(out.iterdir()) if p.suffix in ('.so','.c','.i','.d') or p.name.endswith('_projection')}
symbols={p.name:run(['/usr/bin/readelf','--dyn-syms','--wide',str(p)]).decode() for p in sorted(out.glob('*.so'))}
result={'compiler_sha256':sha(pathlib.Path('/usr/bin/clang').read_bytes()),'header_inputs':header_inputs,'layouts':layouts,'symbols':symbols,'version':1,'inventory_sha256':sha(pathlib.Path(sys.argv[1]).read_bytes()),'compiler':run(['/usr/bin/clang','--version']).decode(),'target':run(['/usr/bin/clang','-dumpmachine']).decode().strip(),'projections':projections,'asts':asts,'artifacts':artifacts,'traces':traces}
(out/'commands.json').write_text(json.dumps(commands,indent=2));(out/'observation.json').write_text(json.dumps(result,indent=2));print(json.dumps(result))
