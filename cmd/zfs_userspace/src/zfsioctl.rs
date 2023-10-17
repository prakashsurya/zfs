use std::ffi::CStr;
use std::ffi::CString;
use std::iter;
use std::mem::size_of;

use nix::errno::Errno;
use nix::ioctl_readwrite_bad;
use nix::libc::c_int;
use nix::unistd::Gid;
use nix::unistd::Group;
use nix::unistd::Uid;
use nix::unistd::User;

const MAXPATHLEN: usize = 4096;
const MAXNAMELEN: usize = 256;
const ZFS_IOC_USERSPACE_MANY: usize = 0x5a2e;

#[derive(Debug, Clone)]
#[repr(C)]
pub struct zfs_cmd_t {
    zc_name: [u8; MAXPATHLEN],
    zc_nvlist_src: u64, // really (char *)
    zc_nvlist_src_size: u64,
    zc_nvlist_dst: u64, // really (char *)
    zc_nvlist_dst_size: u64,
    zc_nvlist_dst_filled: bool,
    zc_pad2: u32,
    zc_history: u64, // really (char *)
    zc_value: [u8; MAXPATHLEN * 2],
    zc_string: [u8; MAXNAMELEN],
    zc_guid: u64,
    zc_nvlist_conf: u64, // really (char *)
    zc_nvlist_conf_size: u64,
    zc_cookie: u64,
    zc_objset_type: u64,
    zc_perm_action: u64,
    zc_history_len: u64,
    zc_history_offset: u64,
    zc_obj: u64,
    zc_iflags: u64,
    zc_pad: [u8; 1024 * 1024],
}

impl Default for zfs_cmd_t {
    fn default() -> Self {
        zfs_cmd_t {
            zc_name: [0u8; MAXPATHLEN],
            zc_nvlist_src: 0,
            zc_nvlist_src_size: 0,
            zc_nvlist_dst: 0,
            zc_nvlist_dst_size: 0,
            zc_nvlist_dst_filled: false,
            zc_pad2: 0,
            zc_history: 0,
            zc_value: [0u8; MAXPATHLEN * 2],
            zc_string: [0u8; MAXNAMELEN],
            zc_guid: 0,
            zc_nvlist_conf: 0,
            zc_nvlist_conf_size: 0,
            zc_cookie: 0,
            zc_objset_type: 0,
            zc_perm_action: 0,
            zc_history_len: 0,
            zc_history_offset: 0,
            zc_obj: 0,
            zc_iflags: 0,
            zc_pad: [0u8; 1024 * 1024],
        }
    }
}

#[derive(Debug, Clone)]
#[repr(C)]
pub struct zfs_useracct_t {
    zu_domain: [u8; 256],
    zu_rid: u32,
    zu_pad: u32,
    zu_space: u64,
}

impl Default for zfs_useracct_t {
    fn default() -> Self {
        zfs_useracct_t {
            zu_domain: [0u8; 256],
            zu_rid: 0,
            zu_pad: 0,
            zu_space: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct UserAcct {
    pub domain: String,
    pub rid: u32,
    pub space: u64,
}

pub enum NameType {
    User,
    Group,
    Project,
}

impl UserAcct {
    pub fn name_string(&self, name_type: NameType, id_to_name: bool) -> String {
        if self.domain.is_empty() {
            if id_to_name {
                let translate = || match name_type {
                    NameType::User => Some(User::from_uid(Uid::from_raw(self.rid)).ok()??.name),
                    NameType::Group => Some(Group::from_gid(Gid::from_raw(self.rid)).ok()??.name),
                    NameType::Project => None,
                };
                translate().unwrap_or_else(|| format!("{}", self.rid))
            } else {
                format!("{}", self.rid)
            }
        } else {
            // SMB
            // XXX translate to string name
            format!("{}-{}", self.domain, self.rid)
        }
    }
}

impl From<&zfs_useracct_t> for UserAcct {
    fn from(z: &zfs_useracct_t) -> Self {
        let str = CStr::from_bytes_until_nul(&z.zu_domain)
            .unwrap()
            .to_str()
            .unwrap();
        UserAcct {
            domain: str.to_string(),
            rid: z.zu_rid,
            space: z.zu_space,
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum UserQuotaProp {
    UserUsed,
    UserQuota,
    GroupUsed,
    GroupQuota,
    UserObjUsed,
    UserObjQuota,
    GroupObjUsed,
    GroupObjQuota,
    ProjectUsed,
    ProjectQuota,
    ProjectObjUsed,
    ProjectObjQuota,
}

impl UserQuotaProp {
    pub fn name_type(&self) -> NameType {
        match self {
            UserQuotaProp::UserUsed
            | UserQuotaProp::UserQuota
            | UserQuotaProp::UserObjUsed
            | UserQuotaProp::UserObjQuota => NameType::User,
            UserQuotaProp::GroupUsed
            | UserQuotaProp::GroupQuota
            | UserQuotaProp::GroupObjUsed
            | UserQuotaProp::GroupObjQuota => NameType::Group,
            UserQuotaProp::ProjectUsed
            | UserQuotaProp::ProjectQuota
            | UserQuotaProp::ProjectObjUsed
            | UserQuotaProp::ProjectObjQuota => NameType::Project,
        }
    }
}

ioctl_readwrite_bad!(zfs_ioc_userspace_many, ZFS_IOC_USERSPACE_MANY, zfs_cmd_t);

/// Get the user/group/project space accounting associated with the specified dataset.  If an error
/// is encountered, the returned iterator will terminate.
///
/// XXX the provided fd must remain open for the duration of the iterator.
///
/// XXX need better error handling, return iterator of Results?
pub fn zfs_userspace(
    dev_zfs_fd: i32,
    dataset: &str,
    prop: UserQuotaProp,
) -> impl Iterator<Item = UserAcct> {
    let name = CString::new(dataset).unwrap();
    let mut cookie = 0u64;
    let x = iter::from_fn(move || {
        // XXX kernel is going to modify buf, need to do some magic unsafe to indicate?
        let mut buf = vec![zfs_useracct_t::default(); 1024];
        let mut cmd = zfs_cmd_t {
            zc_objset_type: prop as u64,
            zc_nvlist_dst: buf.as_ptr() as u64,
            zc_cookie: cookie,
            ..Default::default()
        };
        cmd.zc_name[..name.as_bytes().len()].copy_from_slice(name.as_bytes());
        cmd.zc_nvlist_dst_size = (buf.len() * size_of::<zfs_useracct_t>()) as u64;
        let res = unsafe { zfs_ioc_userspace_many(dev_zfs_fd, &mut cmd) };
        cookie = cmd.zc_cookie;
        if res.is_err() {
            return None;
        }
        if cmd.zc_nvlist_dst_size == 0 {
            return None;
        }
        let iter = buf
            .into_iter()
            .take(cmd.zc_nvlist_dst_size as usize / size_of::<zfs_useracct_t>())
            .map(|ref useracct| useracct.into());
        Some(iter)
    });
    x.flatten()
}

/*
fn zfs_ioc_userspace_many(fd: i32, cmd: &mut zfs_cmd_t) -> i32 {
    unsafe { ioctl(fd, ZFS_IOC_USERSPACE_MANY, cmd) }
}
*/

/*
typedef enum {
    ZFS_PROP_USERUSED,
    ZFS_PROP_USERQUOTA,
    ZFS_PROP_GROUPUSED,
    ZFS_PROP_GROUPQUOTA,
    ZFS_PROP_USEROBJUSED,
    ZFS_PROP_USEROBJQUOTA,
    ZFS_PROP_GROUPOBJUSED,
    ZFS_PROP_GROUPOBJQUOTA,
    ZFS_PROP_PROJECTUSED,
    ZFS_PROP_PROJECTQUOTA,
    ZFS_PROP_PROJECTOBJUSED,
    ZFS_PROP_PROJECTOBJQUOTA,
    ZFS_NUM_USERQUOTA_PROPS
} zfs_userquota_prop_t;

    ZFS_IOC_USERSPACE_MANY,			/* 0x5a2e */

typedef struct zfs_useracct {
    char zu_domain[256];
    uid_t zu_rid;
    uint32_t zu_pad;
    uint64_t zu_space;
} zfs_useracct_t;


struct zfs_cmd {
    char		zc_name[MAXPATHLEN];	/* name of pool or dataset */
    uint64_t	zc_nvlist_src;		/* really (char *) */
    uint64_t	zc_nvlist_src_size;
    uint64_t	zc_nvlist_dst;		/* really (char *) */
    uint64_t	zc_nvlist_dst_size;
    boolean_t	zc_nvlist_dst_filled;	/* put an nvlist in dst? */
    int		zc_pad2;

    /*
     * The following members are for legacy ioctls which haven't been
     * converted to the new method.
     */
    uint64_t	zc_history;		/* really (char *) */
    char		zc_value[MAXPATHLEN * 2];
    char		zc_string[MAXNAMELEN];
    uint64_t	zc_guid;
    uint64_t	zc_nvlist_conf;		/* really (char *) */
    uint64_t	zc_nvlist_conf_size;
    uint64_t	zc_cookie;
    uint64_t	zc_objset_type;
    uint64_t	zc_perm_action;
    uint64_t	zc_history_len;
    uint64_t	zc_history_offset;
    uint64_t	zc_obj;
    uint64_t	zc_iflags;		/* internal to zfs(7fs) */
    zfs_share_t	zc_share;
    dmu_objset_stats_t zc_objset_stats;
    struct drr_begin zc_begin_record;
    zinject_record_t zc_inject_record;
    uint32_t	zc_defer_destroy;
    uint32_t	zc_flags;
    uint64_t	zc_action_handle;
    int		zc_cleanup_fd;
    uint8_t		zc_simple;
    uint8_t		zc_pad[3];		/* alignment */
    uint64_t	zc_sendobj;
    uint64_t	zc_fromobj;
    uint64_t	zc_createtxg;
    zfs_stat_t	zc_stat;
    uint64_t	zc_zoneid;
} zfs_cmd_t;
*/
