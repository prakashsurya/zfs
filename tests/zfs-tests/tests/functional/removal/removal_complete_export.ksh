#! /bin/ksh -p
# SPDX-License-Identifier: CDDL-1.0
#
# CDDL HEADER START
#
# This file and its contents are supplied under the terms of the
# Common Development and Distribution License ("CDDL"), version 1.0.
# You may only use this file in accordance with the terms of version
# 1.0 of the CDDL.
#
# A full copy of the text of the CDDL should have accompanied this
# source.  A copy of the CDDL is also available via the Internet at
# http://www.illumos.org/license/CDDL.
#
# CDDL HEADER END
#

#
# Copyright (c) 2026 by Delphix. All rights reserved.
#

. $STF_SUITE/include/libtest.shlib
. $STF_SUITE/tests/functional/removal/removal.kshlib

#
# DESCRIPTION:
#
# Destroying (or exporting) a pool while a device removal is in its
# finalization phase must not race with the removal thread.
#
# vdev_remove_complete() calls spa_vdev_enter() twice and clears
# svr_thread between the two calls (in vdev_remove_replace_with_indirect).
# spa_export_common(), via spa_async_suspend -> spa_vdev_remove_suspend,
# only waits until svr_thread is cleared.  Without additional coordination
# it can therefore set spa_export_thread in the window between the two
# spa_vdev_enter() calls, tripping the ASSERT0P(spa->spa_export_thread) in
# the second one.
#
# This test reproduces that window deterministically using two injection
# tunables:
#   - zfs_removal_complete_delay_ms parks the removal thread between its
#     two spa_vdev_enter() calls (and sets zfs_removal_in_complete).
#   - zfs_export_complete_delay_ms holds the exporter with spa_export_thread
#     set while the removal thread is parked.
# The pool destroy is issued once the removal thread is parked, so when it
# resumes it runs its second spa_vdev_enter() with spa_export_thread set.
# On a fixed kernel spa_export_common() waits for the removal finalization
# to complete and the destroy succeeds; on a broken kernel the removal
# thread panics.
#
# NB: reproducing the failure on a broken kernel relies on tripping
# ASSERT0P(spa->spa_export_thread), which is only compiled in on a debug
# (--enable-debug) build.  On a non-debug kernel the broken code would not
# panic, so this test only guards the race on debug builds.
#
# STRATEGY:
#
# 1. Create a pool with one disk and write some data to it.
# 2. Add a second disk to receive the evacuated data.
# 3. Arm both injections and start removing the first disk.
# 4. Wait (via zfs_removal_in_complete) until the removal thread is parked
#    between its two spa_vdev_enter() calls.
# 5. Destroy the pool -- the export sets spa_export_thread and is held.
# 6. The destroy must succeed (and not panic the kernel).
#

function cleanup
{
	set_tunable32 REMOVAL_COMPLETE_DELAY_MS 0
	set_tunable32 EXPORT_COMPLETE_DELAY_MS 0
	poolexists $TESTPOOL && destroy_pool $TESTPOOL
}

log_onexit cleanup

log_must default_setup_noexit "$REMOVEDISK"
log_must zfs set compression=off $TESTPOOL/$TESTFS

#
# Write some data that will be evacuated from the device when we start
# the removal.
#
log_must dd if=/dev/urandom of=$TESTDIR/$TESTFILE0 bs=1M count=256

#
# Add the second device where the data will be evacuated to.
#
log_must zpool add -f $TESTPOOL $NOTREMOVEDISK

#
# Arm the injections:
#   - park the removal thread for 10s between its two spa_vdev_enter() calls
#   - hold the exporter for 30s after it sets spa_export_thread
# The export hold comfortably outlasts the removal park, so the removal
# thread's second spa_vdev_enter() runs while spa_export_thread is set.
#
log_must set_tunable32 REMOVAL_COMPLETE_DELAY_MS 10000
log_must set_tunable32 EXPORT_COMPLETE_DELAY_MS 30000

log_must zpool remove $TESTPOOL $REMOVEDISK

#
# Wait until the removal thread is parked between its two spa_vdev_enter()
# calls.
#
typeset -i tries=0
while [[ "$(get_tunable REMOVAL_IN_COMPLETE)" != "1" ]]; do
	((tries++))
	if ((tries > 1200)); then
		log_fail "Removal thread never reached the completion window"
	fi
	sleep 0.1
done

#
# Destroy the pool while the removal thread is parked.  The export sets
# spa_export_thread and is held by EXPORT_COMPLETE_DELAY_MS; on a fixed
# kernel it then waits for the removal finalization to finish and the
# destroy succeeds.  On a broken kernel the removal thread panics in
# spa_vdev_enter().
#
log_must zpool destroy $TESTPOOL

log_pass "Pool destroy during removal finalization completed cleanly"
