use anchor_lang::prelude::*;
use litesvm::{types::TransactionResult, LiteSVM};
use solana_sdk::{
    clock::Clock, instruction::Instruction, native_token::LAMPORTS_PER_SOL, pubkey::Pubkey,
    signature::Keypair, signer::Signer, transaction::Transaction,
};

pub struct SvmContext {
    pub svm: LiteSVM,
    pub payer: Keypair,
}

impl SvmContext {
    pub fn new() -> Self {
        let mut svm = LiteSVM::new();
        let payer = gen_and_fund_key(&mut svm);
        Self { svm, payer }
    }

    pub fn update_blockhash(&mut self) {
        self.svm.expire_blockhash();
    }

    pub fn submit_transaction(
        &mut self,
        ixs: &[Instruction],
        signers: &[&Keypair],
    ) -> TransactionResult {
        self.update_blockhash();
        let tx = Transaction::new_signed_with_payer(
            ixs,
            Some(&self.payer.pubkey()),
            &[&[&self.payer], signers].concat(),
            self.svm.latest_blockhash(),
        );
        self.svm.send_transaction(tx)
    }

    pub fn clock(&self) -> Clock {
        self.svm.get_sysvar::<solana_program::clock::Clock>()
    }

    pub fn set_clock(&mut self, unix_timestamp: i64) {
        let clock = self.svm.get_sysvar::<solana_program::clock::Clock>();
        let new_clock = Clock {
            unix_timestamp,
            ..clock
        };
        self.svm.set_sysvar(&new_clock);
    }

    pub fn load_and_deserialize<T: AccountDeserialize>(&self, address: &Pubkey) -> T {
        let account = self.svm.get_account(address).unwrap();
        T::try_deserialize(&mut account.data.as_slice()).unwrap()
    }

    pub fn gen_and_fund_key(&mut self) -> Keypair {
        gen_and_fund_key(&mut self.svm)
    }
}

pub fn gen_and_fund_key(svm: &mut LiteSVM) -> Keypair {
    let keypair = Keypair::new();
    let pubkey = keypair.pubkey();
    svm.airdrop(&pubkey, 10 * LAMPORTS_PER_SOL).unwrap();
    keypair
}
