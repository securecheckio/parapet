import { FC, useEffect, useState } from 'react';
import { useParams, Link } from 'react-router-dom';
import { ArrowLeft, CheckCircle, Circle } from 'lucide-react';
import { useWallet } from '@solana/wallet-adapter-react';
import { apiService } from '../services/api';

interface Lesson {
  id: string;
  title: string;
  description: string;
  content: string;
}

interface Course {
  id: string;
  title: string;
  description: string;
  slug: string;
  content: {
    lessons: Lesson[];
  };
}

interface Progress {
  id: string;
  course_id: string;
  completed: boolean;
  progress_data: {
    completed_lessons?: string[];
  };
}

const CourseDetail: FC = () => {
  const { slug } = useParams<{ slug: string }>();
  const { connected } = useWallet();
  const [course, setCourse] = useState<Course | null>(null);
  const [progress, setProgress] = useState<Progress | null>(null);
  const [currentLessonIndex, setCurrentLessonIndex] = useState(0);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    if (slug) {
      loadCourse();
      if (connected) {
        loadProgress();
      }
    }
  }, [slug, connected]);

  const loadCourse = async () => {
    try {
      const data = await apiService.getCourseBySlug(slug!);
      setCourse(data);
    } catch (error) {
      console.error('Failed to load course:', error);
    } finally {
      setLoading(false);
    }
  };

  const loadProgress = async () => {
    if (!course) return;
    
    try {
      const data = await apiService.getCourseProgress(course.id);
      setProgress(data);
    } catch (error) {
      console.log('Progress not available');
    }
  };

  const markLessonComplete = async (lessonId: string) => {
    if (!course || !connected) return;

    const completedLessons = progress?.progress_data.completed_lessons || [];
    if (completedLessons.includes(lessonId)) return;

    const newCompletedLessons = [...completedLessons, lessonId];
    const allLessonsCompleted = newCompletedLessons.length === course.content.lessons.length;

    try {
      const data = await apiService.updateCourseProgress(course.id, {
        progress_data: { completed_lessons: newCompletedLessons },
        completed: allLessonsCompleted,
      });
      setProgress(data);
    } catch (error) {
      console.error('Failed to update progress:', error);
    }
  };

  const isLessonCompleted = (lessonId: string) => {
    return progress?.progress_data.completed_lessons?.includes(lessonId) || false;
  };

  const goToNextLesson = () => {
    if (course && currentLessonIndex < course.content.lessons.length - 1) {
      setCurrentLessonIndex(currentLessonIndex + 1);
    }
  };

  const goToPrevLesson = () => {
    if (currentLessonIndex > 0) {
      setCurrentLessonIndex(currentLessonIndex - 1);
    }
  };

  if (loading) {
    return (
      <div className="flex items-center justify-center min-h-screen">
        <div className="text-lg text-slate-400">Loading course...</div>
      </div>
    );
  }

  if (!course) {
    return (
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-12 text-center">
        <h2 className="text-2xl font-bold text-white mb-4">Course Not Found</h2>
        <Link to="/learn" className="text-blue-400 hover:text-blue-300">
          ← Back to courses
        </Link>
      </div>
    );
  }

  const currentLesson = course.content.lessons[currentLessonIndex];
  const progressPercent = progress 
    ? ((progress.progress_data.completed_lessons?.length || 0) / course.content.lessons.length) * 100 
    : 0;

  return (
    <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
      {/* Header */}
      <div className="mb-8">
        <Link 
          to="/learn" 
          className="inline-flex items-center gap-2 text-slate-400 hover:text-slate-200 transition-colors mb-6"
        >
          <ArrowLeft size={20} />
          Back to courses
        </Link>

        <div className="flex items-start justify-between mb-4">
          <div>
            <h1 className="text-3xl sm:text-4xl font-bold text-white mb-2">
              {course.title}
            </h1>
            <p className="text-slate-400">{course.description}</p>
          </div>
          {progress?.completed && (
            <div className="flex items-center gap-2 px-4 py-2 bg-green-500/10 border border-green-500/30 rounded-lg">
              <CheckCircle className="text-green-400" size={20} />
              <span className="text-sm text-green-400 font-medium">Completed</span>
            </div>
          )}
        </div>

        {/* Progress Bar */}
        {connected && progress && (
          <div className="bg-slate-900/50 border border-slate-800 rounded-lg p-4">
            <div className="flex items-center justify-between mb-2">
              <span className="text-sm text-slate-400">Your Progress</span>
              <span className="text-sm text-slate-300 font-medium">
                {Math.round(progressPercent)}%
              </span>
            </div>
            <div className="h-2 bg-slate-800 rounded-full overflow-hidden">
              <div
                className="h-full bg-gradient-to-r from-blue-500 to-purple-500 transition-all duration-500"
                style={{ width: `${progressPercent}%` }}
              />
            </div>
          </div>
        )}
      </div>

      <div className="grid lg:grid-cols-4 gap-8">
        {/* Lesson Sidebar */}
        <div className="lg:col-span-1">
          <div className="bg-white/5 border border-white/10 rounded-lg p-4 sticky top-24">
            <h3 className="text-sm font-semibold text-slate-300 uppercase tracking-wide mb-4">
              Lessons
            </h3>
            <div className="space-y-2">
              {course.content.lessons.map((lesson, index) => {
                const completed = isLessonCompleted(lesson.id);
                const isCurrent = index === currentLessonIndex;

                return (
                  <button
                    key={lesson.id}
                    onClick={() => setCurrentLessonIndex(index)}
                    className={`w-full text-left px-3 py-2 rounded-lg transition-all ${
                      isCurrent
                        ? 'bg-blue-500/20 border border-blue-500/50 text-white'
                        : 'hover:bg-white/5 text-slate-400 hover:text-slate-200'
                    }`}
                  >
                    <div className="flex items-center gap-2">
                      {completed ? (
                        <CheckCircle size={16} className="text-green-400 flex-shrink-0" />
                      ) : (
                        <Circle size={16} className="flex-shrink-0" />
                      )}
                      <span className="text-sm line-clamp-2">{lesson.title}</span>
                    </div>
                  </button>
                );
              })}
            </div>
          </div>
        </div>

        {/* Lesson Content */}
        <div className="lg:col-span-3">
          <div className="bg-white/5 border border-white/10 rounded-lg p-8">
            <div className="flex items-center justify-between mb-6">
              <div className="flex items-center gap-3">
                <div className="px-3 py-1 bg-slate-800 rounded text-xs text-slate-400 font-medium">
                  Lesson {currentLessonIndex + 1} of {course.content.lessons.length}
                </div>
              </div>
              {isLessonCompleted(currentLesson.id) && (
                <div className="flex items-center gap-2 text-green-400 text-sm">
                  <CheckCircle size={16} />
                  Completed
                </div>
              )}
            </div>

            <h2 className="text-2xl sm:text-3xl font-bold text-white mb-4">
              {currentLesson.title}
            </h2>
            
            <p className="text-slate-400 mb-8">
              {currentLesson.description}
            </p>

            {/* Lesson Content (HTML) */}
            <div 
              className="prose prose-invert prose-slate max-w-none
                prose-headings:text-white prose-headings:font-bold
                prose-h3:text-xl prose-h3:mb-4 prose-h3:mt-8
                prose-h4:text-lg prose-h4:mb-3 prose-h4:mt-6
                prose-p:text-slate-300 prose-p:leading-relaxed
                prose-ul:text-slate-300 prose-ul:space-y-2
                prose-li:text-slate-300
                prose-strong:text-white prose-strong:font-semibold"
              dangerouslySetInnerHTML={{ __html: currentLesson.content }}
            />

            {/* Navigation Buttons */}
            <div className="flex items-center justify-between mt-12 pt-8 border-t border-slate-800">
              <button
                onClick={goToPrevLesson}
                disabled={currentLessonIndex === 0}
                className="px-6 py-3 bg-slate-800 text-slate-300 rounded-lg hover:bg-slate-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
              >
                Previous
              </button>

              <div className="flex gap-3">
                {connected && !isLessonCompleted(currentLesson.id) && (
                  <button
                    onClick={() => markLessonComplete(currentLesson.id)}
                    className="px-6 py-3 bg-green-600 text-white rounded-lg hover:bg-green-700 transition-colors"
                  >
                    Mark Complete
                  </button>
                )}

                {currentLessonIndex < course.content.lessons.length - 1 ? (
                  <button
                    onClick={goToNextLesson}
                    className="px-6 py-3 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors"
                  >
                    Next Lesson
                  </button>
                ) : (
                  <Link
                    to="/learn"
                    className="px-6 py-3 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors inline-block"
                  >
                    Back to Courses
                  </Link>
                )}
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};

export default CourseDetail;
