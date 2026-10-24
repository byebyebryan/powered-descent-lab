import copy
import unittest
from compare_runs import join
from lossless import pack,unpack

class Protocols(unittest.TestCase):
    def test_numeric_literals_roundtrip(self):
        raw='{"samples":[{"x":-0.0,"y":1e-20,"s":"quoted \\\" }"},{"x":0.0,"y":2.35,"s":"plain"}]}';pool={};self.assertEqual(unpack(pack(raw,pool),pool),raw)
    def test_join_accounts_for_changes_and_unmatched_without_name_matching(self):
        o={'mission':'m','cohort':'c','physical_sha256':'physical-a','occurrence':0,'variant':'a','outcome':'landed','verified_landing':True,'values':{'fuel':10},'capture_sha256':'source-a'}
        a={'metrics':[{'id':'fuel'}],'observations':[o]};b=copy.deepcopy(a);b['observations'][0].update(mission='renamed',values={'fuel':9},capture_sha256='source-b');b['observations'].append({**o,'physical_sha256':'physical-b'})
        r=join(a,b,'a','a','contract','contract');self.assertEqual((r['matched'],r['scientific_changes'],r['candidate_only']),(1,1,1));self.assertEqual(r['same_captures'],0)
        with self.assertRaises(ValueError):join(a,b,'a','a','old','new')
        b['observations'].append(b['observations'][0])
        with self.assertRaises(ValueError):join(a,b,'a','a','contract','contract')

if __name__=='__main__':unittest.main()
