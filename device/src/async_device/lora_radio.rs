use super::radio::{
    Bandwidth, CodingRate, PhyRxTx, RfConfig, RxQuality, SpreadingFactor, TxConfig,
};
use super::region::constants::DEFAULT_DBM;
use super::Timings;

use lora_phy::mod_params::{BoardType, ChipType, RadioError};
use lora_phy::mod_traits::RadioKind;
use lora_phy::LoRa;

/// Convert the spreading factor for use in the external lora-phy crate
impl From<SpreadingFactor> for lora_phy::mod_params::SpreadingFactor {
    fn from(sf: SpreadingFactor) -> Self {
        match sf {
            SpreadingFactor::_7 => lora_phy::mod_params::SpreadingFactor::_7,
            SpreadingFactor::_8 => lora_phy::mod_params::SpreadingFactor::_8,
            SpreadingFactor::_9 => lora_phy::mod_params::SpreadingFactor::_9,
            SpreadingFactor::_10 => lora_phy::mod_params::SpreadingFactor::_10,
            SpreadingFactor::_11 => lora_phy::mod_params::SpreadingFactor::_11,
            SpreadingFactor::_12 => lora_phy::mod_params::SpreadingFactor::_12,
        }
    }
}

/// Convert the bandwidth for use in the external lora-phy crate
impl From<Bandwidth> for lora_phy::mod_params::Bandwidth {
    fn from(bw: Bandwidth) -> Self {
        match bw {
            Bandwidth::_125KHz => lora_phy::mod_params::Bandwidth::_125KHz,
            Bandwidth::_250KHz => lora_phy::mod_params::Bandwidth::_250KHz,
            Bandwidth::_500KHz => lora_phy::mod_params::Bandwidth::_500KHz,
        }
    }
}

/// Convert the coding rate for use in the external lora-phy crate
impl From<CodingRate> for lora_phy::mod_params::CodingRate {
    fn from(cr: CodingRate) -> Self {
        match cr {
            CodingRate::_4_5 => lora_phy::mod_params::CodingRate::_4_5,
            CodingRate::_4_6 => lora_phy::mod_params::CodingRate::_4_6,
            CodingRate::_4_7 => lora_phy::mod_params::CodingRate::_4_7,
            CodingRate::_4_8 => lora_phy::mod_params::CodingRate::_4_8,
        }
    }
}

/// LoRa radio using the physical layer API in the external lora-phy crate
pub struct LoRaRadio<RK> {
    pub(crate) lora: LoRa<RK>,
}

impl<RK> LoRaRadio<RK>
where
    RK: RadioKind + 'static,
{
    pub fn new(lora: LoRa<RK>) -> Self {
        Self { lora }
    }
}

/// Provide the timing values for boards supported by the external lora-phy crate
impl<RK> Timings for LoRaRadio<RK>
where
    RK: RadioKind + 'static,
{
    fn get_rx_window_offset_ms(&self) -> i32 {
        match self.lora.get_board_type() {
            BoardType::Rak4631Sx1262 => -15,
            BoardType::Stm32l0Sx1276 => -15,
            BoardType::Stm32wlSx1262 => -50,
            _ => -50,
        }
    }
    fn get_rx_window_duration_ms(&self) -> u32 {
        match self.lora.get_board_type() {
            BoardType::Rak4631Sx1262 => 1050,
            BoardType::Stm32l0Sx1276 => 1003,
            BoardType::Stm32wlSx1262 => 1050,
            _ => 1050,
        }
    }
}

/// Provide the LoRa physical layer rx/tx interface for boards supported by the external lora-phy crate
impl<RK> PhyRxTx for LoRaRadio<RK>
where
    RK: RadioKind + 'static,
{
    type PhyError = RadioError;

    async fn tx(&mut self, config: TxConfig, buffer: &[u8]) -> Result<u32, Self::PhyError> {
        let mdltn_params = self.lora.create_modulation_params(
            config.rf.spreading_factor.into(),
            config.rf.bandwidth.into(),
            config.rf.coding_rate.into(),
            config.rf.frequency,
        )?;
        let mut tx_pkt_params =
            self.lora
                .create_tx_packet_params(8, false, true, false, &mdltn_params)?;
        let pw = match self.lora.get_board_type().into() {
            ChipType::Sx1276 | ChipType::Sx1277 | ChipType::Sx1278 | ChipType::Sx1279 => {
                if config.pw > DEFAULT_DBM {
                    DEFAULT_DBM
                } else {
                    config.pw
                }
            }
            _ => config.pw,
        };
        self.lora
            .prepare_for_tx(&mdltn_params, pw.into(), false)
            .await?;
        self.lora
            .tx(&mdltn_params, &mut tx_pkt_params, buffer, 0xffffff)
            .await?;
        Ok(0)
    }

    async fn rx(
        &mut self,
        config: RfConfig,
        receiving_buffer: &mut [u8],
    ) -> Result<(usize, RxQuality), Self::PhyError> {
        let mdltn_params = self.lora.create_modulation_params(
            config.spreading_factor.into(),
            config.bandwidth.into(),
            config.coding_rate.into(),
            config.frequency,
        )?;
        let rx_pkt_params = self.lora.create_rx_packet_params(
            8,
            false,
            receiving_buffer.len() as u8,
            true,
            true,
            &mdltn_params,
        )?;
        self.lora
            .prepare_for_rx(
                &mdltn_params,
                &rx_pkt_params,
                None,
                true, // RX continuous
                false,
                4,
                0x00ffffffu32,
            )
            .await?;
        match self.lora.rx(&rx_pkt_params, receiving_buffer).await {
            Ok((received_len, rx_pkt_status)) => {
                Ok((
                    received_len as usize,
                    RxQuality::new(rx_pkt_status.rssi, rx_pkt_status.snr as i8), // downcast snr
                ))
            }
            Err(err) => Err(err),
        }
    }
}
