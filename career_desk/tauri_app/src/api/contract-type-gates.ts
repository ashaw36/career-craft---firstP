import type{CheckUpdateDto,CommandMap,DownloadUpdateDto,InstallUpdateDto,LearningDto,LearningItemDto,MatchStatusUpdateDto,SkillDto,SkillResourceDto}from"./contracts";
const resource:SkillResourceDto={resourceId:"r",skillId:"s",kind:"course",title:"T",source:"official",url:"https://example.com",estimatedHours:1};
const skill:SkillDto={skillId:"s",name:"Skill",category:"technical",description:"D",aliases:[],prerequisiteSkillIds:[],level:1,resources:[resource]};
const item:LearningItemDto={itemId:"i",skillId:"s",title:"T",resourceUrl:null,resourceKind:"course",estimatedHours:1,status:"pending",source:"official",version:1,completionNote:null,convertedExperienceId:null};
void[skill,item];
// @ts-expect-error skillId is a mandatory server contract field
const missingSkillId:SkillDto={name:"Skill",category:"technical",description:"D",aliases:[],prerequisiteSkillIds:[],level:1,resources:[]};
// @ts-expect-error learning path core fields cannot be omitted
const partialLearning:LearningDto={pathId:"p",items:[]};
// @ts-expect-error arbitrary Json index fields are forbidden on strict skill DTOs
const arbitrarySkill:SkillDto={...skill,unexpected:"value"};
void[missingSkillId,partialLearning,arbitrarySkill];
const matchUpdate:CommandMap["update_match_status"]["request"]={matchId:"m",status:"applied",expectedVersion:1};
// @ts-expect-error stale-write protection requires expectedVersion
const unsafeMatchUpdate:CommandMap["update_match_status"]["request"]={matchId:"m",status:"applied"};
void[matchUpdate,unsafeMatchUpdate];
const statusUpdate:MatchStatusUpdateDto={id:"m",trackingStatus:"applied",eventId:2,version:4};
// @ts-expect-error status update response is not a full MatchDto and has no score
const statusWithMatchFields:MatchStatusUpdateDto={...statusUpdate,matchScore:90};
// @ts-expect-error every CAS response includes the new version
const statusWithoutVersion:MatchStatusUpdateDto={id:"m",trackingStatus:"applied",eventId:2};
void[statusUpdate,statusWithMatchFields,statusWithoutVersion];
const updateAvailable:CheckUpdateDto={available:true,version:"1.2.0",body:null,date:null};const updateMissing:CheckUpdateDto={available:false};const updateDownload:DownloadUpdateDto={staged:true,version:"1.2.0",bytes:42};const updateInstall:InstallUpdateDto={installed:true,relaunchRequired:true,version:"1.2.0"};
// @ts-expect-error available update requires body and date metadata
const unsafeUpdateAvailable:CheckUpdateDto={available:true,version:"1.2.0"};
// @ts-expect-error download response has an exact shape
const unsafeUpdateDownload:DownloadUpdateDto={staged:true,version:null,bytes:1,available:true};
void[updateAvailable,updateMissing,updateDownload,updateInstall,unsafeUpdateAvailable,unsafeUpdateDownload];
