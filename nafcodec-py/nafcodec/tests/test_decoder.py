import gzip
import os
import tempfile
import unittest

import nafcodec
from . import data

try:
    try:
        from importlib.resources import files
    except ImportError:
        from importlib_resources import files  # type: ignore
except ImportError:
    files = None  # type: ignore

class TestDecoder(unittest.TestCase):

    @unittest.skipUnless(files, "importlib.resources not found")
    def test_fastq(self):
        path = os.fspath(files(data).joinpath("phix.naf"))
        decoder = nafcodec.Decoder(path)
        records = list(decoder)
        self.assertEqual(len(records), 42)
        self.assertEqual(records[0].id, "SRR1377138.1")
        self.assertEqual(records[0].sequence[:36], "NGCTCTTAAACCTGCTATTGAGGCTTGTGGCATTTC")
        self.assertEqual(records[0].quality[:31], "#8CCCGGGGGGGGGGGGGGGGGGGGGGGGGG")

    @unittest.skipUnless(files, "importlib.resources not found")
    def test_dna(self):
        path = os.fspath(files(data).joinpath("NZ_AAEN01000029.naf"))
        decoder = nafcodec.Decoder(path)
        records = list(decoder)
        self.assertEqual(len(records), 30)
        self.assertEqual(records[0].id, "NZ_AAEN01000029.1")
        self.assertEqual(records[0].sequence.count('A'), 62115)
        self.assertEqual(records[0].sequence.count('C'), 28747)
        self.assertEqual(records[0].sequence.count('G'), 30763)
        self.assertEqual(records[0].sequence.count('T'), 61152)
        self.assertIs(records[0].quality, None)

    @unittest.skipUnless(files, "importlib.resources not found")
    def test_protein(self):
        path = os.fspath(files(data).joinpath("LuxC.naf"))
        decoder = nafcodec.Decoder(path)
        records = list(decoder)
        self.assertEqual(len(records), 12)
        self.assertEqual(records[0].id, "sp|P19841|LUXC_PHOPO")
        self.assertEqual(records[0].sequence[:25], "MCNAEFKGDCMIKKIPMIIGGAERD")
        self.assertIs(records[0].quality, None)
        self.assertEqual(records[5].id, "sp|P29236|LUXC2_PHOLE")
        self.assertEqual(records[5].sequence[:25], "MIKKIPMIIGGVVQNTSGYGMRELT")
        self.assertIs(records[5].quality, None)

    @unittest.skipUnless(files, "importlib.resources not found")
    def test_dna_masked(self):
        path = os.fspath(files(data).joinpath("masked.naf"))
        decoder = nafcodec.Decoder(path)
        records = list(decoder)
        self.assertEqual(len(records), 2)
        self.assertEqual(records[0].id, "test1")
        self.assertTrue(records[0].sequence[:657].isupper())
        self.assertTrue(records[0].sequence[657:676].islower())
        self.assertTrue(records[0].sequence[676:1311].isupper())
        self.assertTrue(records[0].sequence[1311:1350].islower())
        self.assertIs(records[0].quality, None)
