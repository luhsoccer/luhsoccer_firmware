/**
 * \file
 *
 * \brief Enhanced Embedded Flash Controller (EEFC) driver for SAM.
 *
 * Copyright (c) 2011-2018 Microchip Technology Inc. and its subsidiaries.
 *
 * \asf_license_start
 *
 * \page License
 *
 * Subject to your compliance with these terms, you may use Microchip
 * software and any derivatives exclusively with Microchip products.
 * It is your responsibility to comply with third party license terms applicable
 * to your use of third party software (including open source software) that
 * may accompany Microchip software.
 *
 * THIS SOFTWARE IS SUPPLIED BY MICROCHIP "AS IS". NO WARRANTIES,
 * WHETHER EXPRESS, IMPLIED OR STATUTORY, APPLY TO THIS SOFTWARE,
 * INCLUDING ANY IMPLIED WARRANTIES OF NON-INFRINGEMENT, MERCHANTABILITY,
 * AND FITNESS FOR A PARTICULAR PURPOSE. IN NO EVENT WILL MICROCHIP BE
 * LIABLE FOR ANY INDIRECT, SPECIAL, PUNITIVE, INCIDENTAL OR CONSEQUENTIAL
 * LOSS, DAMAGE, COST OR EXPENSE OF ANY KIND WHATSOEVER RELATED TO THE
 * SOFTWARE, HOWEVER CAUSED, EVEN IF MICROCHIP HAS BEEN ADVISED OF THE
 * POSSIBILITY OR THE DAMAGES ARE FORESEEABLE.  TO THE FULLEST EXTENT
 * ALLOWED BY LAW, MICROCHIP'S TOTAL LIABILITY ON ALL CLAIMS IN ANY WAY
 * RELATED TO THIS SOFTWARE WILL NOT EXCEED THE AMOUNT OF FEES, IF ANY,
 * THAT YOU HAVE PAID DIRECTLY TO MICROCHIP FOR THIS SOFTWARE.
 *
 * \asf_license_stop
 *
 */
/*
 * Support and FAQ: visit <a href="https://www.microchip.com/support/">Microchip Support</a>
 */

#include "efc.h"

/**
 * RAM Functions provided by ASF
 * These functions are difficult/impossible to replicate in pure rust as you cannot have any flash accesses
 * during these functions.
 *
 * These functions have been adapted to use a very small subset of ASF headers and be portable across
 * atsam4 chips.
 */

/* Flash Writing Protection Key */
#define FWP_KEY    0x5Au

#define EEFC_FCR_FCMD(value) \
	((EEFC_FCR_FCMD_Msk & ((value) << EEFC_FCR_FCMD_Pos)))
#define EEFC_ERROR_FLAGS  (EEFC_FSR_FLOCKE | EEFC_FSR_FCMDE | EEFC_FSR_FLERR)


/**
 * \brief Perform read sequence. Supported sequences are read Unique ID and
 * read User Signature
 *
 * \param p_efc Pointer to an EFC instance.
 * \param ul_cmd_st Start command to perform.
 * \param ul_cmd_sp Stop command to perform.
 * \param p_ul_buf Pointer to an data buffer.
 * \param ul_size Buffer size.
 * \param p_ul_data Address of flash region being used.
 *                  Usually 0x00400000u (IFLASH0_ADDR or READ_BUFF_ADDR0)
 *
 * \return 0 if successful, otherwise returns an error code.
 */
__attribute__ ((__noinline__))
__attribute__ ((section(".data")))
uint32_t efc_perform_read_sequence(Efc *p_efc,
		uint32_t ul_cmd_st, uint32_t ul_cmd_sp,
		uint32_t *p_ul_buf, uint32_t ul_size, uint32_t *p_ul_data)
{
	volatile uint32_t ul_status;
	uint32_t ul_cnt;

	if (p_ul_buf == NULL) {
		return EFC_RC_INVALID;
	}

	p_efc->EEFC_FMR |= (0x1u << 16);

	/* Send the Start Read command */
	p_efc->EEFC_FCR = EEFC_FCR_FKEY_PASSWD | EEFC_FCR_FARG(0)
			| EEFC_FCR_FCMD(ul_cmd_st);

	/* Wait for the FRDY bit in the Flash Programming Status Register
	 * (EEFC_FSR) falls.
	 */
	do {
		ul_status = p_efc->EEFC_FSR;
	} while ((ul_status & EEFC_FSR_FRDY) == EEFC_FSR_FRDY);

	/* The data is located in the first address of the Flash
	 * memory mapping.
	 */
	for (ul_cnt = 0; ul_cnt < ul_size; ul_cnt++) {
		p_ul_buf[ul_cnt] = p_ul_data[ul_cnt];
	}

	/* To stop the read mode */
	p_efc->EEFC_FCR =
			EEFC_FCR_FKEY_PASSWD | EEFC_FCR_FARG(0) |
			EEFC_FCR_FCMD(ul_cmd_sp);

	/* Wait for the FRDY bit in the Flash Programming Status Register (EEFC_FSR)
	 * rises.
	 */
	do {
		ul_status = p_efc->EEFC_FSR;
	} while ((ul_status & EEFC_FSR_FRDY) != EEFC_FSR_FRDY);

	p_efc->EEFC_FMR &= ~(0x1u << 16);

	return EFC_RC_OK;
}

/**
 * \brief Perform command.
 *
 * \param p_efc Pointer to an EFC instance.
 * \param ul_fcr Flash command.
 *
 * \return The current status.
 */
__attribute__ ((__noinline__))
__attribute__ ((section(".data")))
uint32_t efc_perform_fcr(Efc *p_efc, uint32_t ul_fcr)
{
	volatile uint32_t ul_status;

	p_efc->EEFC_FCR = ul_fcr;
	do {
		ul_status = p_efc->EEFC_FSR;
	} while ((ul_status & EEFC_FSR_FRDY) != EEFC_FSR_FRDY);

	return (ul_status & EEFC_ERROR_FLAGS);
}
