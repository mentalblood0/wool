require "woollib/common"
require "woollib/Sweater"
require "woollib/exceptions"

require "./users/Users"

module Wool
  class Service
    mserializable

    getter sweater : Sweater
    getter users : Users

    def initialize(@sweater, @users)
    end

    enum Error
      PseudonymNotFound     = 0
      OperationNotPermitted = 1
    end

    def answer(s : Users::Site, pseudonym : String, c : Command(Sweater) | Command(Users))
      u = (@users.get s, pseudonym).not_nil! rescue return Error::PseudonymNotFound
      case u.role
      when User::Role::User
        case c
        when Command::Get, Command::GetRelations, Command::GetByTags
          return c.exec @sweater
        when Command(Users)
          return Error::OperationNotPermitted
        else
          return @users.push u.id, c
        end
      when User::Role::Moderator
        case c
        when Command(Sweater)
          return c.exec @sweater
        when Command(Users)
          return c.exec @users
        end
      end
    end
  end
end
