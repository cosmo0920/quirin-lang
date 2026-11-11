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

"""Strict replay primitives for one existing-file Linux observation profile.

The native observer and exclusive process fixture are trusted. Addresses may
vary, but live allocation separation, generation ownership and ring edges may
not. This adapter never supplies a static certificate.
"""
import errno
from check_teardown_trace import require
from check_file_teardown_trace import LAYOUT

PLAN = [('calloc',136),('strdup',5),('strdup',28),('malloc',38),
        ('malloc',88),('strdup',9),('malloc',96),('strdup',8),('malloc',50),('calloc',120)]


class Replay:
    def __init__(self, records, case):
        self.records, self.case, self.cursor = records, case, 0
        self.live, self.released = {}, []

    def peek(self, field):
        require(self.cursor < len(self.records), 'truncated trace')
        return self.records[self.cursor].get(field)

    def consume(self, kind, **fields):
        require(self.cursor < len(self.records), 'truncated trace')
        require(self.records[self.cursor] == dict(case=self.case, sequence=self.cursor, kind=kind, **fields),
                f'case {self.case}: unexpected {kind} at {self.cursor}')
        self.cursor += 1

    def enter(self, name): self.consume('enter', function=name)
    def leave(self, name): self.consume('exit', function=name)

    def release(self, ident):
        require(ident in self.live, 'release of non-live generation')
        self.consume('free', id=ident)
        del self.live[ident]
        self.released.append(ident)

    def separated(self, address, size):
        require(type(address) is int and 0 < address and address + size <= 2**64, 'invalid storage')
        require(all(address + size <= a or a + n <= address for a,n in self.live.values()),
                'overlapping live storage')

    def allocations(self):
        for ident,(allocator,size) in enumerate(PLAN,1):
            address = self.peek('address')
            self.separated(address,size)
            self.consume('allocation',id=ident,address=address,size=size,allocator=allocator)
            self.live[ident] = (address,size)
            if ident == 4: self.release(4)

    def ring(self, parent, offset, owner=0, node_offset=''):
        sentinel = self.live[parent][0] + LAYOUT[offset]
        address = self.live[owner][0] + LAYOUT[node_offset] if owner else sentinel
        return dict(sentinel=sentinel,next=address,prev=address,
                    nodes=[dict(owner=owner,address=address,next=sentinel,prev=sentinel)] if owner else [])

    def snapshot(self, chunk=None, up=True, counter=None):
        # `up` is observed state-ring membership; `counter` is the independent
        # context total. They coincide unless an observation says otherwise.
        owner = 7 if chunk else 0
        stream = dict(id=5,name=6,context=1,memory=0,
            ring=self.ring(5,'stream_ring',owner,'chunk_node'),
            up_ring=self.ring(5,'up_ring',owner if up else 0,'state_node'),
            down_ring=self.ring(5,'down_ring',owner if not up else 0,'state_node'),
            chunks=[chunk] if chunk else [])
        if counter is None: counter = int(bool(chunk) and up)
        self.consume('snapshot',context=dict(id=1,total=int(bool(chunk)),up=counter,
            fields=[0,0,2,0,0,3],ring=self.ring(1,'context_ring',5,'stream_node'),streams=[stream]))

    def backend_release(self, fd, fs_size, mapping=0, size=0, data_size=0, up=0):
        self.release(9)
        self.consume('backend_release',id=10,fd=fd,map=mapping,size=size,fs_size=fs_size,
                     data_size=data_size,up=up)
        self.release(10)

    def close(self, fd, phase, injected=False, epoch=1):
        self.consume('close',phase=phase,fd=fd,epoch=epoch,result=-1 if injected else 0,
                     errno=errno.EIO if injected else 0,injected=int(injected))

    def unmap(self, mapping, size, phase, epoch=1, result=0, error=0):
        self.consume('unmap',phase=phase,map=mapping,size=size,epoch=epoch,result=result,errno=error)

    def parents(self, chunk_cleanup=None):
        for name in ('cio_destroy','cio_stream_destroy_all','cio_stream_destroy','cio_chunk_close_stream'):
            self.enter(name)
        if chunk_cleanup:
            self.enter('cio_chunk_close'); self.enter('cio_file_close')
            chunk_cleanup()
            self.leave('cio_file_close'); self.release(8)
            self.consume('chunk_state',id=7,primary_cleared=1,state_cleared=1,total=1)
            self.release(7)
            self.enter('cio_chunk_counter_total_sub'); self.leave('cio_chunk_counter_total_sub')
            self.consume('counter',total=0,up=0); self.leave('cio_chunk_close')
        self.leave('cio_chunk_close_stream'); self.release(6)
        self.consume('stream_state',id=5,primary_cleared=1,chunks_empty=1,up_empty=1,down_empty=1)
        self.release(5); self.leave('cio_stream_destroy'); self.leave('cio_stream_destroy_all')
        self.release(2); self.release(3)
        self.consume('context_state',id=1,total=0,up=0,streams_empty=1)
        self.release(1); self.leave('cio_destroy')

    def finish(self):
        require(self.cursor == len(self.records) and not self.live
                and sorted(self.released) == list(range(1,11)), 'terminal trace/allocation mismatch')
