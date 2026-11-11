// SPDX-License-Identifier: Apache-2.0
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#include "native_file_profile_observer.h"
int main(int argc,char **argv) {
    assert(argc==3);unsigned scenario=(unsigned)strtoul(argv[1],NULL,10);assert(scenario<7);
    fail_chmod=scenario==1 || scenario==2;fail_close=scenario==2;
    profile_init(argv[2],300+scenario,"begin_existing");
    int error=-999;
    struct cio_chunk *ch=cio_chunk_open(context,streams[0],"chunk-0",scenario>=5?CIO_OPEN_RD:CIO_OPEN_RW,64,&error);
    if(ch)chunks[chunk_count++]=ch;
    prefix("constructor_result");printf(",\"chunk\":%u,\"error\":%d}\n",identity(ch),error);
    fixture_probe("returned");snapshot();profile_destroy();
    if(scenario==2) {
        errno=0;int result=__real_close(opened_fd);int error=errno;
        errno=0;int fd_result=fcntl(opened_fd,F_GETFD);int fd_error=errno;
        prefix("harness_close");printf(",\"fd\":%d,\"epoch\":%u,\"result\":%d,\"errno\":%d,\"fd_result\":%d,\"fd_errno\":%d}\n",opened_fd,fd_epoch,result,error,fd_result,fd_error);
        assert(result==0 && fd_result==-1 && fd_error==EBADF);
    }
    return 0;
}
